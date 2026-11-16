//! The initial Python policy: spacing in safe comments only.

use std::{borrow::Cow, ops::Range};

use super::{
    BreakOpportunity, LanguageFormatError, LanguageFormatPolicy, SpacingRules, TextEdit,
    python_markup::safe_comment_body_ranges,
};
use crate::{
    core::physical_source::PhysicalSource,
    parser::{Grammar, parse},
    spacing::spacing_edits,
};
use tree_sitter::Node;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PythonFormatPolicy;

#[derive(Debug)]
struct Comment {
    range: Range<usize>,
    row: usize,
    indentation: String,
    standalone: bool,
}

impl LanguageFormatPolicy for PythonFormatPolicy {
    fn plan_spacing_edits(
        &self,
        source: &PhysicalSource<'_>,
        rules: &SpacingRules,
    ) -> Result<Vec<TextEdit>, LanguageFormatError> {
        let text = source.text();
        validate_python_encoding(text)?;
        let (parse_text, bom_bytes) = text
            .strip_prefix('\u{feff}')
            .map_or((text, 0), |without_bom| {
                (without_bom, '\u{feff}'.len_utf8())
            });
        let parse_text = normalize_bare_carriage_returns(parse_text);
        let tree = parse(Grammar::Python, &parse_text)
            .map_err(|error| LanguageFormatError::Policy(error.to_string()))?;
        let root = tree.root_node();
        if root.has_error() || contains_missing_node(root) {
            return Err(LanguageFormatError::Policy(
                "Python syntax parse failed".to_owned(),
            ));
        }

        let comments = collect_comments(root, source, bom_bytes);
        let mut edits = Vec::new();
        let mut index = 0;
        while index < comments.len() {
            let comment = &comments[index];
            if !comment.standalone {
                plan_comment_region_edits(text, std::slice::from_ref(comment), rules, &mut edits);
                index += 1;
                continue;
            }

            let group_start = index;
            index += 1;
            while index < comments.len()
                && comments[index].standalone
                && comments[index].row == comments[index - 1].row + 1
                && comments[index].indentation == comments[index - 1].indentation
            {
                index += 1;
            }
            let group = &comments[group_start..index];
            plan_comment_region_edits(text, group, rules, &mut edits);
        }
        Ok(edits)
    }

    fn plan_break_opportunities(
        &self,
        _source: &PhysicalSource<'_>,
    ) -> Result<Vec<BreakOpportunity>, LanguageFormatError> {
        Ok(Vec::new())
    }
}

fn collect_comments(root: Node<'_>, source: &PhysicalSource<'_>, bom_bytes: usize) -> Vec<Comment> {
    let mut nodes = Vec::new();
    collect_comment_nodes(root, &mut nodes);
    nodes.sort_by_key(|node| node.start_byte());

    nodes
        .into_iter()
        .filter_map(|node| {
            let range = (node.start_byte() + bom_bytes)..(node.end_byte() + bom_bytes);
            let line = source.lines().nth(node.start_position().row)?;
            let marker_offset = range.start;
            if marker_offset < line.start_offset() || marker_offset > line.content_end_offset() {
                return None;
            }
            let prefix = &source.text()[line.start_offset()..marker_offset];
            let standalone = prefix
                .chars()
                .all(|character| matches!(character, ' ' | '\t'))
                || (line.index() == 0
                    && prefix.strip_prefix('\u{feff}').is_some_and(|rest| {
                        rest.chars()
                            .all(|character| matches!(character, ' ' | '\t'))
                    }));
            Some(Comment {
                range,
                row: node.start_position().row,
                indentation: prefix.trim_start_matches('\u{feff}').to_owned(),
                standalone,
            })
        })
        .collect()
}

fn collect_comment_nodes<'tree>(node: Node<'tree>, comments: &mut Vec<Node<'tree>>) {
    if node.kind() == "comment" {
        comments.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_comment_nodes(child, comments);
    }
}

fn contains_missing_node(node: Node<'_>) -> bool {
    if node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).any(contains_missing_node)
}

// The grammar uses LF to terminate comments; this same-length parser copy keeps
// source byte ranges intact while letting bare-CR lines parse independently.
fn normalize_bare_carriage_returns(source: &str) -> Cow<'_, str> {
    if !source.as_bytes().contains(&b'\r') {
        return Cow::Borrowed(source);
    }

    let mut normalized = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' && characters.peek() != Some(&'\n') {
            normalized.push('\n');
        } else {
            normalized.push(character);
        }
    }
    Cow::Owned(normalized)
}

fn plan_comment_region_edits(
    source: &str,
    comments: &[Comment],
    rules: &SpacingRules,
    edits: &mut Vec<TextEdit>,
) {
    let bodies = comments
        .iter()
        .filter_map(|comment| {
            comment
                .range
                .start
                .checked_add(1)
                .filter(|start| *start <= comment.range.end)
                .map(|start| start..comment.range.end)
        })
        .collect::<Vec<_>>();
    for range in safe_comment_body_ranges(source, &bodies) {
        let body = &source[range.clone()];
        edits.extend(spacing_edits(rules, body).into_iter().map(|edit| TextEdit {
            range: (range.start + edit.range.start)..(range.start + edit.range.end),
            replacement: edit.replacement,
        }));
    }
}

fn validate_python_encoding(source: &str) -> Result<(), LanguageFormatError> {
    let has_bom = source.starts_with('\u{feff}');
    let source_lines = PhysicalSource::new(source);
    let lines = source_lines
        .lines()
        .take(2)
        .map(|line| line.content())
        .collect::<Vec<_>>();
    let Some(first_line) = lines.first() else {
        return Ok(());
    };
    let first_line = first_line.strip_prefix('\u{feff}').unwrap_or(first_line);
    let declaration = python_encoding_cookie(first_line).or_else(|| {
        if python_line_allows_second_encoding_cookie(first_line) {
            lines.get(1).and_then(|line| python_encoding_cookie(line))
        } else {
            None
        }
    });
    let Some(encoding) = declaration else {
        return Ok(());
    };

    let normalized = encoding
        .chars()
        .filter(|character| !matches!(character, '-' | '_' | '.'))
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if matches!(
        normalized.as_str(),
        "utf8" | "utf8sig" | "u8" | "utf" | "cp65001"
    ) {
        return Ok(());
    }

    let message = if has_bom {
        "UTF-8 BOM conflicts with the Python encoding declaration".to_owned()
    } else {
        format!("Python source encoding {encoding:?} is not supported; use UTF-8")
    };
    Err(LanguageFormatError::Policy(message))
}

// Python only treats line two as a cookie when line one is blank or a comment.
fn python_encoding_cookie(line: &str) -> Option<&str> {
    let line = line.trim_start_matches([' ', '\t', '\x0c']);
    let comment = line.strip_prefix('#')?;
    let declaration = comment.match_indices("coding").find_map(|(index, _)| {
        let suffix = &comment[index + "coding".len()..];
        let suffix = suffix
            .strip_prefix(':')
            .or_else(|| suffix.strip_prefix('='))?;
        Some(suffix.trim_start_matches([' ', '\t']))
    })?;
    let end = declaration
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_alphanumeric() && !"._-".contains(character)).then_some(index)
        })
        .unwrap_or(declaration.len());
    (end > 0).then_some(&declaration[..end])
}

fn python_line_allows_second_encoding_cookie(line: &str) -> bool {
    let line = line.trim_start_matches([' ', '\t', '\x0c']);
    line.is_empty() || line.starts_with('#')
}
