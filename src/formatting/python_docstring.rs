//! Source-preserving extraction of actual Python docstrings.

use std::ops::Range;

use tree_sitter::Node;

use super::{SpacingRules, TextEdit, python_markup::safe_prose_ranges};
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
        if collect_docstring_literals(expression, source, bom_bytes, &mut literals) {
            plan_docstring_prose_edits(source, &literals, bom_bytes, rules, edits);
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

fn string_content_nodes(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() == "string_content")
        .collect()
}

fn plan_docstring_prose_edits(
    source: &str,
    literals: &[Node<'_>],
    bom_bytes: usize,
    rules: &SpacingRules,
    edits: &mut Vec<TextEdit>,
) {
    let mut prose = String::new();
    let mut segments = Vec::new();
    let mut ambiguous = false;
    'literals: for literal in literals {
        let spelling = &source[literal.start_byte() + bom_bytes..literal.end_byte() + bom_bytes];
        let raw = spelling.starts_with(['r', 'R']);
        for content in string_content_nodes(*literal) {
            let mut escapes = Vec::new();
            if raw {
                collect_raw_escape_ranges(
                    source,
                    content.start_byte() + bom_bytes..content.end_byte() + bom_bytes,
                    &mut escapes,
                );
            } else {
                collect_escape_ranges(content, bom_bytes, &mut escapes);
                escapes.sort_by_key(|escape| escape.start);
            }
            let mut start = content.start_byte() + bom_bytes;
            for escape in escapes {
                append_segment(source, start..escape.start, &mut prose, &mut segments);
                let spelling = &source[escape.clone()];
                if !raw && ambiguous_escape(spelling) {
                    ambiguous = true;
                    break 'literals;
                }
                if !raw && spelling == "\\n" {
                    prose.push('\n');
                } else if !raw && spelling == "\\r" {
                    prose.push('\r');
                } else if !raw && matches!(spelling, "\\t" | "\\f" | "\\v") {
                    prose.push('\t');
                } else if !raw
                    && spelling.starts_with(['\\'])
                    && spelling[1..].starts_with(['\r', '\n'])
                {
                    // A physical line continuation contributes no docstring character.
                } else {
                    prose.push_str(spelling);
                }
                start = escape.end;
            }
            append_segment(
                source,
                start..content.end_byte() + bom_bytes,
                &mut prose,
                &mut segments,
            );
        }
    }

    if ambiguous && literals.len() > 1 {
        return;
    }

    for safe in safe_prose_ranges(&prose) {
        for (logical, original_start) in &segments {
            let start = safe.start.max(logical.start);
            let end = safe.end.min(logical.end);
            if start < end {
                let original = original_start + start - logical.start;
                plan_literal_text_edits(source, original..original + end - start, rules, edits);
            }
        }
    }
}

fn ambiguous_escape(spelling: &str) -> bool {
    matches!(
        spelling.as_bytes().get(1),
        Some(b'x' | b'u' | b'U' | b'N' | b'0'..=b'7' | b'\\')
    )
}

fn append_segment(
    source: &str,
    range: Range<usize>,
    prose: &mut String,
    segments: &mut Vec<(Range<usize>, usize)>,
) {
    if !range.is_empty() {
        let start = prose.len();
        prose.push_str(&source[range.clone()]);
        segments.push((start..prose.len(), range.start));
    }
}

fn collect_escape_ranges(node: Node<'_>, bom_bytes: usize, ranges: &mut Vec<Range<usize>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "escape_sequence" {
            ranges.push(child.start_byte() + bom_bytes..child.end_byte() + bom_bytes);
        } else {
            collect_escape_ranges(child, bom_bytes, ranges);
        }
    }
}

fn collect_raw_escape_ranges(source: &str, range: Range<usize>, ranges: &mut Vec<Range<usize>>) {
    let mut cursor = range.start;
    while cursor < range.end {
        if source.as_bytes()[cursor] != b'\\' {
            cursor += source[cursor..].chars().next().unwrap().len_utf8();
            continue;
        }
        let start = cursor;
        cursor += 1;
        if cursor < range.end {
            let kind = source.as_bytes()[cursor];
            cursor += source[cursor..].chars().next().unwrap().len_utf8();
            let digits = match kind {
                b'x' => 2,
                b'u' => 4,
                b'U' => 8,
                b'0'..=b'7' => 2,
                _ => 0,
            };
            for _ in 0..digits {
                if cursor < range.end
                    && if matches!(kind, b'0'..=b'7') {
                        matches!(source.as_bytes()[cursor], b'0'..=b'7')
                    } else {
                        source.as_bytes()[cursor].is_ascii_hexdigit()
                    }
                {
                    cursor += 1;
                } else {
                    break;
                }
            }
            if kind == b'N'
                && source.as_bytes().get(cursor) == Some(&b'{')
                && let Some(end) = source[cursor..range.end].find('}')
            {
                cursor += end + 1;
            }
        }
        ranges.push(start..cursor);
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
