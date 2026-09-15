use std::ops::Range;

use crate::parser::{Grammar, parse};
use tree_sitter::Node;

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    config::{Config, SpacingConfig},
    formatting::{
        BreakOpportunity, LanguageFormatError, TextEdit, validate_break_opportunities,
        validate_text_edits,
    },
    spacing::spacing_edits,
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

const WRAPPING_PROTECTED_NODE_KINDS: &[&str] = &["hard_line_break"];

const PROSE_CONTAINER_KINDS: &[&str] = &[
    "inline",
    "emphasis",
    "strong_emphasis",
    "strikethrough",
    "link_text",
    "image_description",
];

const EMPHASIS_DELIMITER_KINDS: &[&str] = &["emphasis_delimiter"];
const UNORDERED_LIST_MARKER_KINDS: &[&str] =
    &["list_marker_minus", "list_marker_plus", "list_marker_star"];

#[derive(Debug)]
struct WrappingInlineRange {
    range: Range<usize>,
    continuation: String,
}

// These are all named nodes in the pinned block grammar.  Keeping the list
// explicit makes a grammar update fail closed instead of silently treating a
// new construct as ordinary prose.
const BLOCK_NODE_KINDS: &[&str] = &[
    "atx_heading",
    "backslash_escape",
    "block_quote",
    "code_fence_content",
    "document",
    "fenced_code_block",
    "html_block",
    "indented_code_block",
    "info_string",
    "inline",
    "language",
    "link_destination",
    "link_label",
    "link_reference_definition",
    "link_title",
    "list",
    "list_item",
    "list_marker_dot",
    "list_marker_minus",
    "list_marker_parenthesis",
    "list_marker_plus",
    "list_marker_star",
    "paragraph",
    "pipe_table",
    "pipe_table_cell",
    "pipe_table_delimiter_cell",
    "pipe_table_delimiter_row",
    "pipe_table_header",
    "pipe_table_row",
    "section",
    "setext_heading",
    "task_list_marker_checked",
    "task_list_marker_unchecked",
    "thematic_break",
    "atx_h1_marker",
    "atx_h2_marker",
    "atx_h3_marker",
    "atx_h4_marker",
    "atx_h5_marker",
    "atx_h6_marker",
    "block_continuation",
    "block_quote_marker",
    "entity_reference",
    "fenced_code_block_delimiter",
    "minus_metadata",
    "numeric_character_reference",
    "pipe_table_align_left",
    "pipe_table_align_right",
    "plus_metadata",
    "setext_h1_underline",
    "setext_h2_underline",
];

// Likewise, this is the named-node allow-list for the inline grammar.  The
// prose collector still only descends through PROSE_CONTAINER_KINDS.
const INLINE_NODE_KINDS: &[&str] = &[
    "backslash_escape",
    "collapsed_reference_link",
    "code_span",
    "code_span_delimiter",
    "email_autolink",
    "emphasis",
    "emphasis_delimiter",
    "entity_reference",
    "full_reference_link",
    "hard_line_break",
    "html_tag",
    "image",
    "image_description",
    "inline",
    "inline_link",
    "latex_block",
    "latex_span_delimiter",
    "link_destination",
    "link_label",
    "link_text",
    "link_title",
    "numeric_character_reference",
    "shortcut_link",
    "strikethrough",
    "strong_emphasis",
    "uri_autolink",
];

/// Plans validated spacing edits for Markdown prose while preserving inline
/// constructs whose contents are not displayed as ordinary prose.
pub(crate) fn plan_edits(config: &Config, source: &str) -> anyhow::Result<Vec<TextEdit>> {
    plan_spacing_edits(source, &config.spacing).map_err(|error| anyhow::anyhow!(error))
}

/// Plans the existing Markdown prose spacing behavior for the policy seam.
///
/// This deliberately shares the same inline traversal used by wrapping.  The
/// spacing engine remains syntax-agnostic; this module owns the CST filtering.
pub(crate) fn plan_spacing_edits(
    source: &str,
    rules: &SpacingConfig,
) -> Result<Vec<TextEdit>, LanguageFormatError> {
    let block_tree = parse(Grammar::Markdown, source)
        .map_err(|error| LanguageFormatError::Policy(error.to_string()))?;
    let mut inline_ranges = Vec::new();
    collect_inline_ranges(block_tree.root_node(), &mut inline_ranges);

    let mut edits = Vec::new();
    for inline_range in inline_ranges {
        let inline_source = source.get(inline_range.clone()).ok_or_else(|| {
            LanguageFormatError::Policy("Markdown inline node has an invalid byte range".into())
        })?;
        let inline_tree = parse(Grammar::MarkdownInline, inline_source)
            .map_err(|error| LanguageFormatError::Policy(error.to_string()))?;

        // Recovery trees can contain misleading prose-looking descendants.
        // Keeping the whole inline node unchanged is safer than formatting a
        // malformed construct partially.
        if inline_tree.root_node().has_error()
            || !is_safe_inline_tree(inline_tree.root_node(), inline_source)
        {
            continue;
        }

        let mut exclusions = Vec::new();
        collect_spacing_protected_inline_ranges(
            inline_tree.root_node(),
            inline_source,
            &mut exclusions,
        );

        for edit in spacing_edits(rules, inline_source) {
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

    validate_text_edits(source, &mut edits)
        .map_err(|error| LanguageFormatError::Policy(error.to_string()))?;
    Ok(edits)
}

/// Plans conservative Markdown soft-break positions against the post-spacing
/// source. Ordinary paragraphs, direct blockquotes, and top-level unordered
/// list items are supported; all other block contexts remain ineligible.
pub(crate) fn plan_break_opportunities(
    source: &str,
) -> Result<Vec<BreakOpportunity>, LanguageFormatError> {
    let block_tree = parse(Grammar::Markdown, source)
        .map_err(|error| LanguageFormatError::Policy(error.to_string()))?;
    let root = block_tree.root_node();
    if root.has_error() || has_missing_node(root) || has_unknown_node(root, BLOCK_NODE_KINDS) {
        return Ok(Vec::new());
    }

    let mut inline_ranges = Vec::new();
    collect_paragraph_inline_ranges(root, false, false, &mut inline_ranges);
    collect_direct_blockquote_inline_ranges(root, source, &mut inline_ranges);
    collect_top_level_unordered_list_inline_ranges(root, source, &mut inline_ranges);
    let mut opportunities = Vec::new();
    for inline_range in inline_ranges {
        let inline_source = source.get(inline_range.range.clone()).ok_or_else(|| {
            LanguageFormatError::Policy("Markdown inline node has an invalid byte range".into())
        })?;
        let inline_tree = parse(Grammar::MarkdownInline, inline_source)
            .map_err(|error| LanguageFormatError::Policy(error.to_string()))?;
        let inline_root = inline_tree.root_node();
        if inline_root.has_error()
            || has_missing_node(inline_root)
            || has_unknown_node(inline_root, INLINE_NODE_KINDS)
            || has_node_kind(inline_root, "hard_line_break")
            // The inline grammar exposes raw HTML delimiters as html_tag
            // nodes but leaves their payload as ordinary text. Treating that
            // text as prose could split a script/style/body payload while
            // preserving only the tags, so protect the whole inline range.
            || has_node_kind(inline_root, "html_tag")
            || !is_safe_inline_tree(inline_root, inline_source)
        {
            continue;
        }

        // A later break can pair an existing pipe-bearing header fragment
        // with a delimiter-looking fragment elsewhere in this paragraph.
        // Without a Markdown layout engine, suppress this whole range rather
        // than trying to prove each combination of generated seams harmless.
        if contains_pipe_table_like_syntax(inline_source) {
            continue;
        }

        let mut prose_ranges = Vec::new();
        if !collect_prose_ranges(inline_root, &mut prose_ranges) {
            continue;
        }
        let mut protected_ranges = Vec::new();
        collect_protected_inline_ranges(inline_root, inline_source, &mut protected_ranges);
        for range in prose_ranges {
            for unprotected_range in subtract_ranges(range, &protected_ranges) {
                collect_range_opportunities(
                    source,
                    inline_range.range.start + unprotected_range.start
                        ..inline_range.range.start + unprotected_range.end,
                    &inline_range.continuation,
                    &mut opportunities,
                );
            }
        }
    }

    validate_break_opportunities(source, &mut opportunities)?;
    Ok(opportunities)
}

fn collect_paragraph_inline_ranges(
    node: Node<'_>,
    in_paragraph: bool,
    blocked_context: bool,
    ranges: &mut Vec<WrappingInlineRange>,
) {
    let in_paragraph = in_paragraph || node.kind() == "paragraph";
    let blocked_context = blocked_context
        || matches!(
            node.kind(),
            "block_quote"
                | "list"
                | "list_item"
                | "atx_heading"
                | "setext_heading"
                | "pipe_table"
                | "pipe_table_cell"
        );
    if node.kind() == "inline" {
        if in_paragraph && !blocked_context {
            ranges.push(WrappingInlineRange {
                range: node.byte_range(),
                continuation: String::new(),
            });
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_paragraph_inline_ranges(child, in_paragraph, blocked_context, ranges);
    }
}

fn collect_direct_blockquote_inline_ranges(
    node: Node<'_>,
    source: &str,
    ranges: &mut Vec<WrappingInlineRange>,
) {
    if node.kind() == "block_quote"
        && node
            .parent()
            .is_some_and(|parent| parent.kind() == "section")
    {
        let mut cursor = node.walk();
        let children = node.named_children(&mut cursor).collect::<Vec<_>>();
        if let [marker, paragraph] = children.as_slice()
            && marker.kind() == "block_quote_marker"
            && paragraph.kind() == "paragraph"
            && source
                .get(marker.byte_range())
                .is_some_and(|text| matches!(text, ">" | "> "))
            && let Some(inline) = only_inline_child(*paragraph)
        {
            ranges.push(WrappingInlineRange {
                range: inline.byte_range(),
                continuation: source[marker.byte_range()].to_owned(),
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_direct_blockquote_inline_ranges(child, source, ranges);
    }
}

fn collect_top_level_unordered_list_inline_ranges(
    node: Node<'_>,
    source: &str,
    ranges: &mut Vec<WrappingInlineRange>,
) {
    if node.kind() == "list"
        && node
            .parent()
            .is_some_and(|parent| parent.kind() == "section")
    {
        let mut cursor = node.walk();
        for item in node.named_children(&mut cursor) {
            let mut item_cursor = item.walk();
            let children = item.named_children(&mut item_cursor).collect::<Vec<_>>();
            let [marker, paragraph] = children.as_slice() else {
                continue;
            };
            let Some(marker_text) = source.get(marker.byte_range()) else {
                continue;
            };
            if !UNORDERED_LIST_MARKER_KINDS.contains(&marker.kind())
                || paragraph.kind() != "paragraph"
                || !marker_text
                    .chars()
                    .all(|character| character == ' ' || matches!(character, '-' | '+' | '*'))
                || marker_text
                    .chars()
                    .filter(|&character| matches!(character, '-' | '+' | '*'))
                    .count()
                    != 1
            {
                continue;
            }
            if let Some(inline) = only_inline_child(*paragraph) {
                ranges.push(WrappingInlineRange {
                    range: inline.byte_range(),
                    continuation: " ".repeat(marker_text.len()),
                });
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_top_level_unordered_list_inline_ranges(child, source, ranges);
    }
}

fn only_inline_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let children = node.named_children(&mut cursor).collect::<Vec<_>>();
    match children.as_slice() {
        [inline] if inline.kind() == "inline" => Some(*inline),
        _ => None,
    }
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

fn has_missing_node(node: Node<'_>) -> bool {
    if node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(has_missing_node)
}

fn has_unknown_node(node: Node<'_>, known_kinds: &[&str]) -> bool {
    if !known_kinds.contains(&node.kind()) {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| has_unknown_node(child, known_kinds))
}

fn collect_prose_ranges(node: Node<'_>, ranges: &mut Vec<Range<usize>>) -> bool {
    if !INLINE_NODE_KINDS.contains(&node.kind()) {
        return false;
    }
    if EXCLUDED_NODE_KINDS.contains(&node.kind())
        || WRAPPING_PROTECTED_NODE_KINDS.contains(&node.kind())
        || EMPHASIS_DELIMITER_KINDS.contains(&node.kind())
    {
        return true;
    }
    if !PROSE_CONTAINER_KINDS.contains(&node.kind()) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if !collect_prose_ranges(child, ranges) {
                return false;
            }
        }
        return true;
    }

    let mut children = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        children.push(child);
    }
    let mut offset = node.start_byte();
    for child in children {
        if child.start_byte() > offset {
            ranges.push(offset..child.start_byte());
        }
        if !collect_prose_ranges(child, ranges) {
            return false;
        }
        offset = child.end_byte();
    }
    if offset < node.end_byte() {
        ranges.push(offset..node.end_byte());
    }
    true
}

fn collect_range_opportunities(
    source: &str,
    range: Range<usize>,
    continuation: &str,
    opportunities: &mut Vec<BreakOpportunity>,
) {
    let Some(text) = source.get(range.clone()) else {
        return;
    };
    let mut seams = text
        .grapheme_indices(true)
        .map(|(offset, grapheme)| (range.start + offset, grapheme));
    let mut previous = seams.next();
    while let Some((offset, grapheme)) = previous {
        let Some((next_offset, next_grapheme)) = seams.next() else {
            break;
        };
        let next = next_offset;
        if grapheme.chars().all(is_replaceable_horizontal_whitespace) {
            let mut end = next;
            let mut following = Some((next_offset, next_grapheme));
            while let Some((relative, candidate)) = following {
                if !candidate.chars().all(is_replaceable_horizontal_whitespace) {
                    break;
                }
                end = relative + candidate.len();
                following = seams.next();
            }
            if is_safe_break_position(source, range.start, offset, end) {
                opportunities.push(BreakOpportunity {
                    replace: offset..end,
                    continuation: continuation.to_owned(),
                });
            }
            previous = following;
        } else {
            if !next_grapheme
                .chars()
                .all(is_replaceable_horizontal_whitespace)
                && is_safe_break_position(source, range.start, next, next)
            {
                opportunities.push(BreakOpportunity {
                    replace: next..next,
                    continuation: continuation.to_owned(),
                });
            }
            previous = Some((next_offset, next_grapheme));
        }
    }
}

fn is_safe_break_position(
    source: &str,
    prose_start: usize,
    break_start: usize,
    break_end: usize,
) -> bool {
    if break_start <= prose_start
        || break_end < break_start
        || break_end >= source.len()
        || !source.is_char_boundary(break_start)
        || !source.is_char_boundary(break_end)
    {
        return false;
    }
    let Some(previous) = source[..break_start].chars().next_back() else {
        return false;
    };
    let Some(following) = source[break_end..].chars().next() else {
        return false;
    };
    if matches!(previous, '\r' | '\n') || matches!(following, '\r' | '\n') {
        return false;
    }
    // A backslash immediately before the generated newline becomes Markdown's
    // escaped hard break. The opportunity must not change that line meaning.
    if previous == '\\' {
        return false;
    }
    // A break between adjacent pipe markers can turn an ordinary paragraph
    // into a pipe-table header and delimiter row after the next safe break.
    if previous == '|' && following == '|' {
        return false;
    }

    // Check the two physical-line fragments created by this opportunity.
    // Looking only at `following` misses a delimiter at the end of the first
    // fragment: replacing its trailing whitespace with a newline can turn
    // that fragment into a thematic break or setext underline.
    let line_start = source[..break_start]
        .rfind(['\r', '\n'])
        .map_or(0, |offset| offset + 1);
    let line_end = source[break_end..]
        .find(['\r', '\n'])
        .map_or(source.len(), |offset| break_end + offset);
    if is_standalone_markdown_delimiter(&source[line_start..break_start]) {
        return false;
    }

    // A generated line beginning with one of these markers can become a
    // heading, list, quote, fence, setext heading, or thematic break.
    // Omitting the seam is safer than trying to infer continuation syntax
    // from prose alone.
    if matches!(
        following,
        '#' | '>' | '-' | '+' | '*' | '_' | '=' | '`' | '~'
    ) {
        return false;
    }
    // Keep the suffix calculation coupled to the physical-line check above:
    // an opportunity must leave an actual non-empty continuation fragment.
    if break_end >= line_end {
        return false;
    }
    // A line beginning with an HTML-looking prefix may become an HTML block,
    // including when the candidate is malformed or not recognized by the
    // grammar. Do not guess whether the source intended prose here: §14's
    // no-reinterpretation rule requires the unknown case to remain unchanged.
    if following == '<' {
        return false;
    }
    // An ordered-list marker is also syntax only when its delimiter is
    // followed by whitespace or the end of the line. Do not reject ordinary
    // prose beginning with a number merely because it starts a line.
    if following.is_ascii_digit() {
        let suffix = &source[break_end..];
        let digits_end = suffix
            .char_indices()
            .take_while(|(_, character)| character.is_ascii_digit())
            .map(|(index, character)| index + character.len_utf8())
            .last()
            .unwrap_or(0);
        if let Some(delimiter) = suffix[digits_end..].chars().next()
            && matches!(delimiter, '.' | ')')
        {
            let after_delimiter = digits_end + delimiter.len_utf8();
            if suffix[after_delimiter..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
            {
                return false;
            }
        }
    }
    true
}

fn is_standalone_markdown_delimiter(fragment: &str) -> bool {
    let delimiters = fragment
        .chars()
        .filter(|&character| !is_horizontal_whitespace(character));
    let Some(delimiter) = delimiters.clone().next() else {
        return false;
    };
    let minimum = match delimiter {
        '=' | '-' => 1,
        '*' | '_' => 3,
        _ => return false,
    };
    delimiters.count() >= minimum
        && fragment
            .chars()
            .all(|character| is_horizontal_whitespace(character) || character == delimiter)
}

fn contains_pipe_table_like_syntax(text: &str) -> bool {
    // A pipe-table delimiter row needs at least one pipe and a cell made only
    // of optional alignment colons, optional whitespace, and three or more
    // hyphens. Refraining from all wraps in such a paragraph is conservative,
    // but ordinary prose containing a lone pipe or non-delimiter cell remains
    // eligible for wrapping.
    text.contains('|') && text.split('|').any(is_pipe_table_delimiter_cell)
}

fn is_pipe_table_delimiter_cell(cell: &str) -> bool {
    let cell = cell.trim_matches(is_horizontal_whitespace);
    let cell = cell.strip_prefix(':').unwrap_or(cell);
    let cell = cell.strip_suffix(':').unwrap_or(cell);
    cell.len() >= 3 && cell.bytes().all(|byte| byte == b'-')
}

fn is_horizontal_whitespace(character: char) -> bool {
    character.is_whitespace() && !matches!(character, '\r' | '\n')
}

// Markdown wrapping may replace only ordinary ASCII spacing. Unicode spaces
// can be glue or word-joiners, so replacing them could change visible text.
fn is_replaceable_horizontal_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t')
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

fn collect_spacing_protected_inline_ranges(
    root: Node<'_>,
    source: &str,
    ranges: &mut Vec<Range<usize>>,
) {
    collect_exclusion_ranges(root, ranges);
    // Preserve the established spacing behavior for malformed candidates:
    // look through later inline ranges and physical lines for the closing
    // delimiter. The wrapping pass deliberately uses a narrower, stronger
    // policy because a generated newline can change block structure.
    collect_unrecognized_autolink_ranges(root, source, ranges, false, true);
    merge_ranges(ranges);
}

fn collect_protected_inline_ranges(root: Node<'_>, source: &str, ranges: &mut Vec<Range<usize>>) {
    collect_exclusion_ranges(root, ranges);
    collect_unrecognized_autolink_ranges(root, source, ranges, true, false);
    collect_unrecognized_html_block_ranges(root, source, ranges);
    merge_ranges(ranges);
}

fn collect_unrecognized_html_block_ranges(
    root: Node<'_>,
    source: &str,
    ranges: &mut Vec<Range<usize>>,
) {
    let mut search_start = 0;
    while let Some(relative_start) = source[search_start..].find('<') {
        let start = search_start + relative_start;
        if is_potential_html_block_start(&source[start..])
            && !has_exclusion_covering(root, start..start + 1)
        {
            let end = source[start..]
                .find(['\r', '\n'])
                .map_or(source.len(), |relative_end| start + relative_end);
            ranges.push(start..end);
        }
        search_start = start + 1;
    }
}

fn is_potential_html_block_start(source: &str) -> bool {
    if source.starts_with("<!--") || source.starts_with("<?") || source.starts_with("<![CDATA[") {
        return true;
    }
    if let Some(declaration) = source.strip_prefix("<!") {
        return declaration
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase());
    }

    let Some(after_open) = source.strip_prefix('<') else {
        return false;
    };
    let after_slash = after_open.strip_prefix('/').unwrap_or(after_open);
    let tag_end = after_slash
        .char_indices()
        .find(|(_, character)| !character.is_ascii_alphabetic())
        .map_or(after_slash.len(), |(offset, _)| offset);
    if tag_end == 0 {
        return false;
    }
    // The grammar already handles recognized HTML tags. For a recovery or
    // future tag, an alphabetic name is enough to make a line-start split
    // syntax-sensitive; keeping a second block-tag table here would not add a
    // distinct safety guarantee.
    match after_slash[tag_end..].chars().next() {
        None | Some('>') | Some('/') => true,
        Some(character) => character.is_whitespace(),
    }
}

fn collect_unrecognized_autolink_ranges(
    root: Node<'_>,
    source: &str,
    ranges: &mut Vec<Range<usize>>,
    protect_unclosed: bool,
    search_across_lines: bool,
) {
    let mut search_start = 0;
    while let Some(relative_start) = source[search_start..].find('<') {
        let start = search_start + relative_start;
        let search_end = if search_across_lines {
            source.len()
        } else {
            source[start..]
                .find(['\r', '\n'])
                .map_or(source.len(), |offset| start + offset)
        };
        let suffix = &source[start + 1..search_end];
        let Some(close_offset) = suffix.find('>') else {
            // Once a URI/email-shaped candidate has no closing delimiter, the
            // grammar cannot tell where its malformed construct ends. Protect
            // the complete remaining inline range instead of splitting at the
            // first whitespace and reclassifying its trailing payload as prose.
            if protect_unclosed
                && is_autolink_candidate(suffix)
                && !has_exclusion_covering(root, start..search_end)
            {
                ranges.push(start..search_end);
            }
            search_start = search_end;
            continue;
        };
        let end = start + 1 + close_offset + 1;
        let candidate = &source[start..end];
        if is_autolink_candidate(&candidate[1..candidate.len() - 1])
            && !has_exclusion_covering(root, start..end)
        {
            ranges.push(start..end);
        }
        search_start = end;
    }
}

fn is_autolink_candidate(candidate: &str) -> bool {
    candidate.contains('@') || candidate.contains("://") || has_uri_scheme(candidate)
}

fn has_uri_scheme(candidate: &str) -> bool {
    let Some((scheme, _)) = candidate.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
        && characters
            .all(|character| character.is_ascii_alphanumeric() || "+-.".contains(character))
}

fn subtract_ranges(range: Range<usize>, protected: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut cursor = range.start;
    for exclusion in protected {
        if exclusion.end <= cursor {
            continue;
        }
        if exclusion.start >= range.end {
            break;
        }
        if exclusion.start > cursor {
            result.push(cursor..exclusion.start.min(range.end));
        }
        cursor = cursor.max(exclusion.end);
        if cursor >= range.end {
            break;
        }
    }
    if cursor < range.end {
        result.push(cursor..range.end);
    }
    result
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
        let mut formatted = source.to_string();
        let edits = plan_edits(&config(alphabets, digits), source).unwrap();
        for edit in edits.into_iter().rev() {
            formatted.replace_range(edit.range, &edit.replacement);
        }
        formatted
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
    fn returns_document_edits_in_ascending_source_order() {
        let edits = plan_edits(
            &config(SpacingRule::Require, SpacingRule::Require),
            "漢A\n漢1\nA漢",
        )
        .unwrap();
        assert_eq!(
            edits
                .iter()
                .map(|edit| edit.range.clone())
                .collect::<Vec<_>>(),
            vec![3..3, 8..8, 11..11]
        );
    }

    #[test]
    fn rejects_invalid_and_overlapping_edits() {
        let mut invalid = vec![TextEdit {
            range: 1..2,
            replacement: String::new(),
        }];
        assert!(validate_text_edits("漢", &mut invalid).is_err());

        let mut overlapping = vec![
            TextEdit {
                range: 0..1,
                replacement: String::new(),
            },
            TextEdit {
                range: 0..0,
                replacement: " ".to_string(),
            },
        ];
        assert!(validate_text_edits("漢", &mut overlapping).is_err());
    }

    #[test]
    fn setext_delimiter_detection_matches_commonmark_minimums() {
        assert!(is_standalone_markdown_delimiter("="));
        assert!(is_standalone_markdown_delimiter("-"));
        assert!(is_standalone_markdown_delimiter("---"));
        assert!(!is_standalone_markdown_delimiter("**"));
        assert!(is_standalone_markdown_delimiter("***"));
    }

    #[test]
    fn unknown_node_kinds_are_ineligible_for_wrapping() {
        assert!(has_unknown_node(
            parse(Grammar::Markdown, "").unwrap().root_node(),
            &["not_the_document_kind"]
        ));
        assert!(!has_unknown_node(
            parse(Grammar::Markdown, "").unwrap().root_node(),
            BLOCK_NODE_KINDS
        ));
        assert!(has_unknown_node(
            parse(Grammar::MarkdownInline, "").unwrap().root_node(),
            &["not_the_inline_kind"]
        ));
        assert!(!has_unknown_node(
            parse(Grammar::MarkdownInline, "plain").unwrap().root_node(),
            INLINE_NODE_KINDS
        ));
    }

    #[test]
    fn validates_and_applies_edits_in_reverse_order() {
        let source = "漢A漢A";
        let mut edits = vec![
            TextEdit {
                range: 3..3,
                replacement: " ".to_string(),
            },
            TextEdit {
                range: 7..7,
                replacement: " ".to_string(),
            },
        ];
        validate_text_edits(source, &mut edits).unwrap();
        let mut formatted = source.to_string();
        for edit in edits.into_iter().rev() {
            formatted.replace_range(edit.range, &edit.replacement);
        }
        assert_eq!(formatted, "漢 A漢 A");
    }
}
