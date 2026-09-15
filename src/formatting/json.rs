//! The built-in JSON formatting policy.

use super::{BreakOpportunity, LanguageFormatError, LanguageFormatPolicy, SpacingRules, TextEdit};
use crate::parser::{Grammar, parse};

/// Plans JSON token-seam wrapping without assigning JSON layout preferences.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct JsonFormatPolicy;

impl LanguageFormatPolicy for JsonFormatPolicy {
    fn plan_spacing_edits(
        &self,
        _source: &str,
        _rules: &SpacingRules,
    ) -> Result<Vec<TextEdit>, LanguageFormatError> {
        // JSON whitespace is syntax, not Markdown prose, so CJK spacing rules
        // must not restyle it.
        Ok(Vec::new())
    }

    fn plan_break_opportunities(
        &self,
        source: &str,
    ) -> Result<Vec<BreakOpportunity>, LanguageFormatError> {
        if serde_json::from_str::<serde_json::Value>(source).is_err()
            || !has_only_known_json_nodes(source)
        {
            return Ok(Vec::new());
        }

        Ok(json_token_seams(source))
    }
}

fn has_only_known_json_nodes(source: &str) -> bool {
    let Ok(tree) = parse(Grammar::Json, source) else {
        return false;
    };
    let root = tree.root_node();
    !root.has_error() && !has_missing_or_unknown_node(root)
}

fn has_missing_or_unknown_node(node: tree_sitter::Node<'_>) -> bool {
    if node.is_missing()
        || !matches!(
            node.kind(),
            "document"
                | "object"
                | "array"
                | "pair"
                | "string"
                | "string_content"
                | "escape_sequence"
                | "number"
                | "true"
                | "false"
                | "null"
                | "{"
                | "}"
                | "["
                | "]"
                | ","
                | ":"
                | "\""
        )
    {
        return true;
    }

    let mut cursor = node.walk();
    node.children(&mut cursor).any(has_missing_or_unknown_node)
}

fn json_token_seams(source: &str) -> Vec<BreakOpportunity> {
    let mut seams = Vec::new();
    let mut cursor = 0;
    let mut previous_end = None;

    while let Some((start, end)) = next_token(source, cursor) {
        if let Some(previous_end) = previous_end {
            let gap = previous_end..start;
            if source[gap.clone()]
                .bytes()
                .all(is_horizontal_json_whitespace)
            {
                seams.push(BreakOpportunity {
                    replace: gap,
                    continuation: String::new(),
                });
            }
        }
        previous_end = Some(end);
        cursor = end;
    }

    seams
}

fn next_token(source: &str, mut cursor: usize) -> Option<(usize, usize)> {
    while source
        .as_bytes()
        .get(cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        cursor += 1;
    }
    let start = cursor;
    let byte = *source.as_bytes().get(cursor)?;
    let end = match byte {
        b'{' | b'}' | b'[' | b']' | b',' | b':' => cursor + 1,
        b'"' => string_end(source, cursor)?,
        b'-' | b'0'..=b'9' => number_end(source, cursor)?,
        b't' if source[cursor..].starts_with("true") => cursor + 4,
        b'f' if source[cursor..].starts_with("false") => cursor + 5,
        b'n' if source[cursor..].starts_with("null") => cursor + 4,
        _ => return None,
    };
    Some((start, end))
}

fn string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    while let Some(&byte) = bytes.get(cursor) {
        match byte {
            b'"' => return Some(cursor + 1),
            b'\\' => {
                cursor += 1;
                match *bytes.get(cursor)? {
                    b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => cursor += 1,
                    b'u' if bytes
                        .get(cursor + 1..cursor + 5)?
                        .iter()
                        .all(u8::is_ascii_hexdigit) =>
                    {
                        cursor += 5;
                    }
                    _ => return None,
                }
            }
            0x20..=0x7f => cursor += 1,
            0x00..=0x1f => return None,
            _ => {
                let character = source[cursor..].chars().next()?;
                cursor += character.len_utf8();
            }
        }
    }
    None
}

fn number_end(source: &str, mut cursor: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(cursor) == Some(&b'-') {
        cursor += 1;
    }
    match *bytes.get(cursor)? {
        b'0' => cursor += 1,
        b'1'..=b'9' => {
            cursor += 1;
            while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }
        }
        _ => return None,
    }
    if bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        let fraction_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == fraction_start {
            return None;
        }
    }
    if matches!(bytes.get(cursor), Some(b'e' | b'E')) {
        cursor += 1;
        if matches!(bytes.get(cursor), Some(b'+' | b'-')) {
            cursor += 1;
        }
        let exponent_start = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor == exponent_start {
            return None;
        }
    }
    Some(cursor)
}

fn is_horizontal_json_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_each_legal_seam_between_json_tokens() {
        let policy = JsonFormatPolicy;
        let source = r#"{"key" : [true, false, null, -12.34]}"#;
        let seams = policy.plan_break_opportunities(source).unwrap();

        assert_eq!(
            seams
                .iter()
                .map(|seam| seam.replace.clone())
                .collect::<Vec<_>>(),
            vec![
                1..1,
                6..7,
                8..9,
                10..10,
                14..14,
                15..16,
                21..21,
                22..23,
                27..27,
                28..29,
                35..35,
                36..36
            ]
        );
    }

    #[test]
    fn rejects_malformed_json_without_planning_edits_or_breaks() {
        let policy = JsonFormatPolicy;
        assert!(
            policy
                .plan_spacing_edits(r#"{"key":"value""#, &SpacingRules::default())
                .unwrap()
                .is_empty()
        );
        assert!(
            policy
                .plan_break_opportunities(r#"{"key":"value""#)
                .unwrap()
                .is_empty()
        );
        assert!(
            policy
                .plan_break_opportunities(r#""first" "second""#)
                .unwrap()
                .is_empty()
        );
    }
}
