use std::ops::Range;

use cjkfmt_parser::{Grammar, parse};
use tree_sitter::Node;

use crate::{
    config::Config,
    spacing::{TextEdit, spacing_edits},
};

const EXCLUDED_NODE_KINDS: &[&str] = &[
    "code_span",
    "link_destination",
    "link_title",
    "link_label",
    "uri_autolink",
    "email_autolink",
    "html_tag",
    "latex_block",
    "entity_reference",
    "numeric_character_reference",
    "backslash_escape",
];

/// Applies configured spacing rules to Markdown prose while preserving inline
/// constructs whose contents are not displayed as ordinary prose.
pub(crate) fn apply_markdown_spacing(config: &Config, source: &str) -> anyhow::Result<String> {
    let block_tree = parse(Grammar::Markdown, source)?;
    let mut inline_ranges = Vec::new();
    collect_inline_ranges(block_tree.root_node(), &mut inline_ranges);

    let mut edits = Vec::new();
    for inline_range in inline_ranges {
        let inline_source = source
            .get(inline_range.clone())
            .ok_or_else(|| anyhow::anyhow!("Markdown inline node has an invalid byte range"))?;
        let inline_tree = parse(Grammar::MarkdownInline, inline_source)?;

        // Recovery trees can contain misleading prose-looking descendants.
        // Keeping the whole inline node unchanged is safer than formatting a
        // malformed construct partially.
        if inline_tree.root_node().has_error()
            || !is_safe_inline_tree(inline_tree.root_node(), inline_source)
        {
            continue;
        }

        let mut exclusions = Vec::new();
        collect_exclusion_ranges(inline_tree.root_node(), &mut exclusions);
        collect_unrecognized_autolink_ranges(
            inline_tree.root_node(),
            inline_source,
            &mut exclusions,
        );
        merge_ranges(&mut exclusions);

        for edit in spacing_edits(config, inline_source) {
            if !exclusions
                .iter()
                .any(|exclusion| edit_intersects(&edit.range, exclusion))
            {
                edits.push(TextEdit {
                    range: (inline_range.start + edit.range.start)
                        ..(inline_range.start + edit.range.end),
                    replacement: edit.replacement,
                });
            }
        }
    }

    apply_text_edits(source, edits)
}

fn collect_inline_ranges(node: Node<'_>, ranges: &mut Vec<Range<usize>>) {
    if node.kind() == "inline" {
        ranges.push(node.byte_range());
        // Do not collect an inline descendant if a future grammar revision
        // happens to nest one: each source slice is parsed exactly once.
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_inline_ranges(child, ranges);
    }
}

fn is_safe_inline_tree(root: Node<'_>, source: &str) -> bool {
    // The pinned grammar recovers some malformed input without setting its
    // ERROR flag (for example, an unclosed code span becomes an empty inline
    // node). Do not reinterpret syntax-looking recovery text as prose.
    let backtick_count = source.chars().filter(|&character| character == '`').count();
    if backtick_count > 0 && (backtick_count % 2 == 1 || !has_node_kind(root, "code_span")) {
        return false;
    }
    let dollar_count = source.chars().filter(|&character| character == '$').count();
    if dollar_count % 2 == 1 {
        return false;
    }
    if source.contains("](") && !has_node_kind(root, "inline_link") && !has_node_kind(root, "image")
    {
        return false;
    }
    true
}

fn collect_unrecognized_autolink_ranges(
    root: Node<'_>,
    source: &str,
    ranges: &mut Vec<Range<usize>>,
) {
    let mut search_start = 0;
    while let Some(relative_start) = source[search_start..].find('<') {
        let start = search_start + relative_start;
        let Some(relative_end) = source[start + 1..].find('>') else {
            break;
        };
        let end = start + 1 + relative_end + 1;
        let candidate = &source[start..end];
        if (candidate.contains('@') || candidate.contains("://"))
            && !has_exclusion_covering(root, start..end)
        {
            ranges.push(start..end);
        }
        search_start = end;
    }
}

fn has_exclusion_covering(node: Node<'_>, range: Range<usize>) -> bool {
    if EXCLUDED_NODE_KINDS.contains(&node.kind()) {
        let node_range = node.byte_range();
        if node_range.start <= range.start && range.end <= node_range.end {
            return true;
        }
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| has_exclusion_covering(child, range.clone()))
}

fn has_node_kind(node: Node<'_>, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| has_node_kind(child, kind))
}

fn collect_exclusion_ranges(node: Node<'_>, ranges: &mut Vec<Range<usize>>) {
    if EXCLUDED_NODE_KINDS.contains(&node.kind()) {
        ranges.push(node.byte_range());
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_exclusion_ranges(child, ranges);
    }
}

fn merge_ranges(ranges: &mut Vec<Range<usize>>) {
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges.drain(..) {
        if let Some(last) = merged.last_mut()
            && range.start <= last.end
        {
            last.end = last.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    *ranges = merged;
}

fn edit_intersects(edit: &Range<usize>, exclusion: &Range<usize>) -> bool {
    if edit.is_empty() {
        exclusion.start <= edit.start && edit.start < exclusion.end
    } else {
        edit.start < exclusion.end && exclusion.start < edit.end
    }
}

fn apply_text_edits(source: &str, mut edits: Vec<TextEdit>) -> anyhow::Result<String> {
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    for edit in &edits {
        if edit.range.start > edit.range.end
            || edit.range.end > source.len()
            || !source.is_char_boundary(edit.range.start)
            || !source.is_char_boundary(edit.range.end)
        {
            anyhow::bail!("spacing edit is not a valid UTF-8 range: {:?}", edit.range);
        }
    }

    for pair in edits.windows(2) {
        let previous = &pair[0].range;
        let current = &pair[1].range;
        if previous.end > current.start
            || (previous.is_empty() && current.is_empty() && previous.start == current.start)
            || (previous.start == current.start && (!previous.is_empty() || !current.is_empty()))
        {
            anyhow::bail!(
                "overlapping spacing edits: {:?} and {:?}",
                previous,
                current
            );
        }
    }

    if edits.is_empty() {
        return Ok(source.to_string());
    }

    let mut formatted = source.to_string();
    for edit in edits.into_iter().rev() {
        formatted.replace_range(edit.range, &edit.replacement);
    }
    Ok(formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SpacingRule;

    fn config(alphabets: SpacingRule, digits: SpacingRule) -> Config {
        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = alphabets;
        config.spacing.digits = digits;
        config
    }

    fn format(source: &str, alphabets: SpacingRule, digits: SpacingRule) -> String {
        apply_markdown_spacing(&config(alphabets, digits), source).unwrap()
    }

    #[test]
    fn formats_prose_inside_inline_markdown_constructs() {
        assert_eq!(
            format(
                "*漢A* **漢1** ~~A漢~~",
                SpacingRule::Require,
                SpacingRule::Require
            ),
            "*漢 A* **漢 1** ~~A 漢~~"
        );
    }

    #[test]
    fn formats_link_text_and_image_description_but_not_destinations() {
        let source = "[漢A](https://example.test/漢A) ![漢A](image漢A.png)";
        assert_eq!(
            format(source, SpacingRule::Require, SpacingRule::Ignore),
            "[漢 A](https://example.test/漢A) ![漢 A](image漢A.png)"
        );
    }

    #[test]
    fn preserves_non_prose_inline_ranges() {
        let source = concat!(
            "`漢A` ``漢1`` <https://example.test/漢A> <foo@example.test> ",
            "<漢A@example.test> <a href=\"漢A\"> $漢A$ &amp;漢A \\*漢A\n",
        );
        assert_eq!(
            format(source, SpacingRule::Require, SpacingRule::Require),
            concat!(
                "`漢A` ``漢1`` <https://example.test/漢A> <foo@example.test> ",
                "<漢A@example.test> <a href=\"漢A\"> $漢A$ &amp;漢 A \\*漢 A\n",
            )
        );
    }

    #[test]
    fn reference_link_label_is_not_formatted() {
        assert_eq!(
            format("[漢A][漢A]", SpacingRule::Require, SpacingRule::Ignore),
            "[漢 A][漢A]"
        );
    }

    #[test]
    fn preserves_fenced_code_and_formats_other_inline_nodes() {
        let source = "漢A\n\n```rust 漢A\n漢A\n```\n\n漢A\n";
        assert_eq!(
            format(source, SpacingRule::Require, SpacingRule::Ignore),
            "漢 A\n\n```rust 漢A\n漢A\n```\n\n漢 A\n"
        );
    }

    #[test]
    fn applies_many_document_edits_using_original_offsets() {
        let source = "漢A\n漢A\n漢A";
        assert_eq!(
            format(source, SpacingRule::Require, SpacingRule::Ignore),
            "漢 A\n漢 A\n漢 A"
        );
    }

    #[test]
    fn prohibit_removes_only_ascii_spaces_in_prose() {
        assert_eq!(
            format("漢  A `漢  A`", SpacingRule::Prohibit, SpacingRule::Ignore),
            "漢A `漢  A`"
        );
    }

    #[test]
    fn malformed_inline_recovery_is_kept_unchanged() {
        for source in ["[漢A](<broken\n漢A>)", "[漢A](broken", "`漢A", "``漢A`"] {
            assert_eq!(
                format(source, SpacingRule::Require, SpacingRule::Ignore),
                source,
                "malformed inline source was changed: {source:?}"
            );
        }
    }

    #[test]
    fn validates_and_applies_edits_in_reverse_order() {
        let source = "漢A漢A";
        let edits = vec![
            TextEdit {
                range: 3..3,
                replacement: " ".to_string(),
            },
            TextEdit {
                range: 7..7,
                replacement: " ".to_string(),
            },
        ];
        assert_eq!(apply_text_edits(source, edits).unwrap(), "漢 A漢 A");
    }
}
