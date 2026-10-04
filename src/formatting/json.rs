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
        if serde_json::from_str::<serde_json::Value>(source).is_err() {
            return Ok(Vec::new());
        }
        let Ok(tree) = parse(Grammar::Json, source) else {
            return Ok(Vec::new());
        };
        let root = tree.root_node();
        if root.has_error() || has_missing_or_unknown_node(root) {
            return Ok(Vec::new());
        }

        Ok(json_token_seams(source, root))
    }
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

const JSON_TOKEN_NODE_KINDS: &[&str] = &[
    "string", "number", "true", "false", "null", "{", "}", "[", "]", ",", ":",
];

fn json_token_seams(source: &str, root: tree_sitter::Node<'_>) -> Vec<BreakOpportunity> {
    let mut tokens = Vec::new();
    collect_json_token_ranges(root, &mut tokens);

    tokens
        .windows(2)
        .filter_map(|pair| {
            let gap = pair[0].end..pair[1].start;
            source[gap.clone()]
                .bytes()
                .all(is_horizontal_json_whitespace)
                .then_some(BreakOpportunity {
                    replace: gap,
                    continuation: String::new(),
                })
        })
        .collect()
}

fn collect_json_token_ranges(
    node: tree_sitter::Node<'_>,
    ranges: &mut Vec<std::ops::Range<usize>>,
) {
    if JSON_TOKEN_NODE_KINDS.contains(&node.kind()) {
        ranges.push(node.byte_range());
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_json_token_ranges(child, ranges);
    }
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
