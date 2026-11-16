//! Source-preserving extraction of actual Python docstrings.

use std::ops::Range;

use tree_sitter::Node;

use super::{SpacingRules, TextEdit};
use crate::spacing::spacing_edits;

pub(crate) fn plan_docstring_edits(
    root: Node<'_>,
    source: &str,
    bom_bytes: usize,
    rules: &SpacingRules,
) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    collect_docstring_edits(root, source, bom_bytes, rules, &mut edits);
    edits
}

fn collect_docstring_edits(
    node: Node<'_>,
    source: &str,
    bom_bytes: usize,
    rules: &SpacingRules,
    edits: &mut Vec<TextEdit>,
) {
    if let Some(expression) = first_docstring_expression(node) {
        let mut literals = Vec::new();
        if collect_docstring_literals(expression, source, bom_bytes, &mut literals)
            && !docstring_contains_protected_markup(source, &literals, bom_bytes)
        {
            for literal in literals {
                plan_literal_edits(source, literal, bom_bytes, rules, edits);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_docstring_edits(child, source, bom_bytes, rules, edits);
    }
}

fn first_docstring_expression(scope: Node<'_>) -> Option<Node<'_>> {
    let body = match scope.kind() {
        "module" => scope,
        "class_definition" | "function_definition" => scope.child_by_field_name("body")?,
        _ => return None,
    };
    let mut cursor = body.walk();
    let first_statement = body
        .named_children(&mut cursor)
        .find(|child| child.kind() != "comment")?;
    let mut expression = first_statement;
    loop {
        if expression.kind() == "parenthesized_expression"
            || expression.kind() == "expression_statement"
        {
            let mut cursor = expression.walk();
            let children = expression
                .named_children(&mut cursor)
                .filter(|child| child.kind() != "comment")
                .collect::<Vec<_>>();
            if children.len() != 1 {
                return None;
            }
            expression = children[0];
        } else {
            return matches!(expression.kind(), "string" | "concatenated_string")
                .then_some(expression);
        }
    }
}

fn collect_docstring_literals<'tree>(
    expression: Node<'tree>,
    source: &str,
    bom_bytes: usize,
    literals: &mut Vec<Node<'tree>>,
) -> bool {
    match expression.kind() {
        "string" if is_qualifying_string(expression, source, bom_bytes) => {
            literals.push(expression);
            true
        }
        "concatenated_string" => {
            let mut cursor = expression.walk();
            let mut valid = true;
            for child in expression.named_children(&mut cursor) {
                if child.kind() == "comment" {
                    continue;
                }
                if !collect_docstring_literals(child, source, bom_bytes, literals) {
                    valid = false;
                    break;
                }
            }
            valid && !literals.is_empty()
        }
        _ => false,
    }
}

fn is_qualifying_string(node: Node<'_>, source: &str, bom_bytes: usize) -> bool {
    let mut cursor = node.walk();
    let Some(start) = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "string_start")
    else {
        return false;
    };
    let range = (start.start_byte() + bom_bytes)..(start.end_byte() + bom_bytes);
    let spelling = &source[range];
    let Some(quote_offset) = spelling.find(['\'', '"']) else {
        return false;
    };
    matches!(
        spelling[..quote_offset].to_ascii_lowercase().as_str(),
        "" | "r" | "u"
    )
}

fn docstring_contains_protected_markup(
    source: &str,
    literals: &[Node<'_>],
    bom_bytes: usize,
) -> bool {
    let mut raw_content = String::new();
    for literal in literals {
        for content in string_content_nodes(*literal) {
            let range = (content.start_byte() + bom_bytes)..(content.end_byte() + bom_bytes);
            raw_content.push_str(&source[range]);
        }
    }
    raw_content.contains('`')
        || raw_content.contains("://")
        || raw_content.contains("mailto:")
        || raw_content.contains("www.")
        || raw_content.contains('[')
        || raw_content.contains(']')
        || raw_content.contains("::")
        || raw_content.split(['\r', '\n']).any(|line| {
            let line = line.trim_start();
            line.starts_with("~~~")
                || line.starts_with("```")
                || line.starts_with(">>>")
                || line.starts_with("...")
                || line.starts_with(".. ")
        })
}

fn string_content_nodes(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() == "string_content")
        .collect()
}

fn plan_literal_edits(
    source: &str,
    literal: Node<'_>,
    bom_bytes: usize,
    rules: &SpacingRules,
    edits: &mut Vec<TextEdit>,
) {
    for content in string_content_nodes(literal) {
        let mut escape_ranges = Vec::new();
        collect_escape_ranges(content, &mut escape_ranges);
        escape_ranges.sort_by_key(|range| range.start_byte());

        let content_start = content.start_byte();
        let content_end = content.end_byte();
        let mut safe_start = content_start;
        for escape in escape_ranges {
            if safe_start < escape.start_byte() {
                plan_literal_text_edits(
                    source,
                    safe_start + bom_bytes..escape.start_byte() + bom_bytes,
                    rules,
                    edits,
                );
            }
            safe_start = escape.end_byte();
        }
        if safe_start < content_end {
            plan_literal_text_edits(
                source,
                safe_start + bom_bytes..content_end + bom_bytes,
                rules,
                edits,
            );
        }
    }
}

fn collect_escape_ranges<'tree>(node: Node<'tree>, ranges: &mut Vec<Node<'tree>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "escape_sequence" {
            ranges.push(child);
        } else {
            collect_escape_ranges(child, ranges);
        }
    }
}

fn plan_literal_text_edits(
    source: &str,
    range: Range<usize>,
    rules: &SpacingRules,
    edits: &mut Vec<TextEdit>,
) {
    let text = &source[range.clone()];
    edits.extend(spacing_edits(rules, text).into_iter().map(|edit| TextEdit {
        range: (range.start + edit.range.start)..(range.start + edit.range.end),
        replacement: edit.replacement,
    }));
}
