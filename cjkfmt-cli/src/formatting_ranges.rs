use std::ops::Range;

use anyhow::bail;
use cjkfmt_parser::Grammar;
use tree_sitter::Node;

use crate::{
    config::Config,
    spacing::{TextEdit, spacing_edits},
};

/// Selects source ranges whose literal text may receive spacing edits.
pub(crate) fn formatting_ranges(
    grammar: Grammar,
    root: Node<'_>,
    source: &str,
) -> anyhow::Result<Vec<Range<usize>>> {
    let mut ranges = Vec::new();
    match grammar {
        Grammar::Python => {
            if root.has_error() {
                return Ok(ranges);
            }
            collect_python(root, source, &mut ranges);
        }
        _ => collect_named(root, "inline", &mut ranges),
    }
    ranges.sort_by_key(|r| (r.start, r.end));
    for range in &ranges {
        if range.start > range.end
            || range.end > source.len()
            || !source.is_char_boundary(range.start)
            || !source.is_char_boundary(range.end)
        {
            bail!("formatting range is not a valid UTF-8 range: {range:?}");
        }
    }
    for pair in ranges.windows(2) {
        if pair[0].end > pair[1].start || pair[0].start == pair[1].start {
            bail!(
                "overlapping formatting ranges: {:?} and {:?}",
                pair[0],
                pair[1]
            );
        }
    }
    Ok(ranges)
}

pub(crate) fn apply_range_spacing(
    config: &Config,
    source: &str,
    ranges: &[Range<usize>],
) -> anyhow::Result<String> {
    let mut edits = Vec::new();
    for range in ranges {
        let text = source
            .get(range.clone())
            .ok_or_else(|| anyhow::anyhow!("invalid formatting range"))?;
        for edit in spacing_edits(config, text) {
            edits.push(TextEdit {
                range: range.start + edit.range.start..range.start + edit.range.end,
                replacement: edit.replacement,
            });
        }
    }
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    for pair in edits.windows(2) {
        if pair[0].range.end > pair[1].range.start || pair[0].range.start == pair[1].range.start {
            bail!("overlapping spacing edits");
        }
    }
    let mut output = source.to_owned();
    for edit in edits.into_iter().rev() {
        output.replace_range(edit.range, &edit.replacement);
    }
    Ok(output)
}

fn collect_named(node: Node<'_>, kind: &str, ranges: &mut Vec<Range<usize>>) {
    if node.kind() == kind {
        ranges.push(node.byte_range());
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_named(child, kind, ranges);
    }
}

fn collect_python(node: Node<'_>, source: &str, ranges: &mut Vec<Range<usize>>) {
    if node.kind() == "comment" {
        let range = node.byte_range();
        ranges.push((range.start + 1).min(range.end)..range.end);
        return;
    }
    if node.kind() == "module" {
        collect_first_docstring(node, source, ranges);
    } else if matches!(node.kind(), "class_definition" | "function_definition")
        && let Some(body) = node.child_by_field_name("body")
    {
        collect_first_docstring(body, source, ranges);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_python(child, source, ranges);
    }
}

fn collect_first_docstring(body: Node<'_>, source: &str, ranges: &mut Vec<Range<usize>>) {
    let mut cursor = body.walk();
    let Some(first) = body
        .named_children(&mut cursor)
        .find(|child| child.kind() != "comment")
    else {
        return;
    };
    let mut node = first;
    while node.kind() == "parenthesized_expression" {
        let mut c = node.walk();
        let Some(next) = node.named_children(&mut c).next() else {
            return;
        };
        node = next;
    }
    if node.kind() == "concatenated_string" {
        let mut c = node.walk();
        let literals: Vec<_> = node.named_children(&mut c).collect();
        // Python only recognizes a concatenation as a docstring when every
        // literal is a text literal. Do not partially format a concatenation
        // containing a bytes or f-string literal.
        if literals
            .iter()
            .all(|literal| literal.kind() == "string" && string_is_eligible(*literal, source))
        {
            for literal in literals {
                collect_string_content(literal, source, ranges);
            }
        }
    } else if node.kind() == "string" && string_is_eligible(node, source) {
        collect_string_content(node, source, ranges);
    }
}

fn string_is_eligible(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    let Some(start) = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "string_start")
    else {
        return false;
    };
    let prefix = &source[start.byte_range()];
    !prefix
        .bytes()
        .any(|b| matches!(b, b'b' | b'B' | b'f' | b'F'))
}

fn collect_string_content(node: Node<'_>, source: &str, ranges: &mut Vec<Range<usize>>) {
    let mut child_cursor = node.walk();
    let Some(start) = node
        .named_children(&mut child_cursor)
        .find(|child| child.kind() == "string_start")
    else {
        return;
    };
    let prefix = &source[start.byte_range()];
    if prefix
        .bytes()
        .any(|b| matches!(b, b'b' | b'B' | b'f' | b'F'))
    {
        return;
    }
    let mut child_cursor = node.walk();
    let Some(content) = node
        .named_children(&mut child_cursor)
        .find(|child| child.kind() == "string_content")
    else {
        return;
    };
    let content_range = content.byte_range();
    let mut cursor = content.walk();
    let mut position = content_range.start;
    for escape in content
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "escape_sequence")
    {
        let r = escape.byte_range();
        if position < r.start {
            ranges.push(position..r.start);
        }
        position = r.end;
    }
    if position < content_range.end {
        ranges.push(position..content_range.end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, SpacingRule};
    use cjkfmt_parser::parse;

    fn config(alphabets: SpacingRule, digits: SpacingRule) -> Config {
        let mut config = Config::default();
        config.spacing.alphabets = alphabets;
        config.spacing.digits = digits;
        config
    }

    fn format(source: &str, config: &Config) -> String {
        let tree = parse(Grammar::Python, source).unwrap();
        let ranges = formatting_ranges(Grammar::Python, tree.root_node(), source).unwrap();
        apply_range_spacing(config, source, &ranges).unwrap()
    }

    #[test]
    fn selects_comments_and_semantic_docstrings_but_not_code_or_other_strings() {
        let source = concat!(
            "\"\"\"漢A\"\"\"\n",
            "value = \"漢A\"\n",
            "async def outer():\n",
            "    # comment漢A\n",
            "    \"\"\"outer漢A\"\"\"\n",
            "    @decorator\n",
            "    class Inner:\n",
            "        \"漢A\"\n",
            "        def method():\n",
            "            \"漢A\"\n",
            "            return f\"漢A\"\n",
            "    return b\"漢A\"\n",
            "\n",
            "def non_doc():\n",
            "    x = 1\n",
            "    \"not a docstringA\"\n",
        );
        let expected = concat!(
            "\"\"\"漢 A\"\"\"\n",
            "value = \"漢A\"\n",
            "async def outer():\n",
            "    # comment 漢 A\n",
            "    \"\"\"outer 漢 A\"\"\"\n",
            "    @decorator\n",
            "    class Inner:\n",
            "        \"漢 A\"\n",
            "        def method():\n",
            "            \"漢 A\"\n",
            "            return f\"漢A\"\n",
            "    return b\"漢A\"\n",
            "\n",
            "def non_doc():\n",
            "    x = 1\n",
            "    \"not a docstringA\"\n",
        );
        let actual = format(source, &config(SpacingRule::Require, SpacingRule::Ignore));
        assert_eq!(actual, expected);
    }

    #[test]
    fn preserves_delimiters_indentation_escapes_and_adjacent_literal_boundaries() {
        let source = concat!(
            "def f():\n",
            "    \"\"\"漢\\nA\\u0041B\n",
            "    C漢\"\"\"\n",
            "    \"漢\" \"A\"\n",
        );
        let expected = concat!(
            "def f():\n",
            "    \"\"\"漢\\nA\\u0041B\n",
            "    C 漢\"\"\"\n",
            "    \"漢\" \"A\"\n",
        );
        assert_eq!(
            format(source, &config(SpacingRule::Require, SpacingRule::Ignore)),
            expected
        );
    }

    #[test]
    fn excludes_concatenated_docstrings_with_fstrings_or_bytes() {
        let source = concat!(
            "def f_string_first():\n",
            "    f\"X\" \"漢A\"\n",
            "def ordinary_first_f_string_second():\n",
            "    \"X\" f\"漢A\"\n",
            "def bytes_first():\n",
            "    b\"X\" \"漢A\"\n",
            "def ordinary_first_bytes_second():\n",
            "    \"X\" b\"漢A\"\n",
        );
        assert_eq!(
            format(source, &config(SpacingRule::Require, SpacingRule::Ignore)),
            source
        );
        assert!(
            formatting_ranges(
                Grammar::Python,
                parse(Grammar::Python, source).unwrap().root_node(),
                source,
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn prohibit_spacing_applies_only_inside_selected_ranges() {
        let source = "# 漢 A\ndef f():\n    \"漢 A\"\n    value = \"漢 A\"\n";
        let expected = "# 漢A\ndef f():\n    \"漢A\"\n    value = \"漢 A\"\n";
        assert_eq!(
            format(source, &config(SpacingRule::Prohibit, SpacingRule::Ignore)),
            expected
        );
    }

    #[test]
    fn malformed_python_is_safe_and_has_no_selected_ranges() {
        let source = "# 漢A\ndef broken(\n";
        let tree = parse(Grammar::Python, source).unwrap();
        assert!(tree.root_node().has_error());
        assert!(
            formatting_ranges(Grammar::Python, tree.root_node(), source)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            format(source, &config(SpacingRule::Require, SpacingRule::Ignore)),
            source
        );
    }

    #[test]
    fn rejects_overlapping_ranges_before_editing() {
        let source = "漢 A";
        let error = apply_range_spacing(
            &config(SpacingRule::Prohibit, SpacingRule::Ignore),
            source,
            &[0..source.len(), 0..source.len()],
        )
        .unwrap_err();
        assert!(error.to_string().contains("overlapping spacing edits"));
    }
}
