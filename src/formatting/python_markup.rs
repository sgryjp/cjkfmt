use std::ops::Range;

use super::python_comment_markup::protected_prose_ranges;

pub(crate) fn safe_comment_body_ranges(source: &str, bodies: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut region = String::new();
    let mut segments = Vec::with_capacity(bodies.len());
    for (index, body) in bodies.iter().enumerate() {
        if index > 0 {
            region.push('\n');
        }
        let logical_start = region.len();
        region.push_str(&source[body.clone()]);
        segments.push((logical_start..region.len(), body.clone()));
    }

    safe_prose_ranges(&region)
        .into_iter()
        .flat_map(|safe| {
            segments.iter().filter_map(move |(logical, original)| {
                let start = safe.start.max(logical.start);
                let end = safe.end.min(logical.end);
                (start < end).then(|| {
                    (original.start + start - logical.start)..(original.start + end - logical.start)
                })
            })
        })
        .collect()
}

pub(crate) fn safe_prose_ranges(prose: &str) -> Vec<Range<usize>> {
    let mut safe_region = Vec::new();
    let mut cursor = 0;
    for protected in protected_prose_ranges(prose) {
        if cursor < protected.start
            && !scan_safe_ranges(prose, cursor..protected.start, &mut safe_region)
        {
            cursor = prose.len();
            break;
        }
        cursor = protected.end;
    }
    if cursor < prose.len() {
        scan_safe_ranges(prose, cursor..prose.len(), &mut safe_region);
    }

    safe_region
}

fn scan_safe_ranges(text: &str, range: Range<usize>, safe: &mut Vec<Range<usize>>) -> bool {
    let mut cursor = range.start;
    let mut safe_start = cursor;

    while cursor < range.end {
        if is_unsupported_line_construct(text, cursor) || text[cursor..].starts_with("::") {
            push_range(safe, safe_start..cursor);
            return false;
        }

        if text.as_bytes()[cursor] == b'<'
            && !is_escaped(text, cursor)
            && url_prefix_length(text, cursor + 1, range.end).is_some()
        {
            push_range(safe, safe_start..cursor);
            let Some(span_end) = angle_url_end(text, cursor, range.end) else {
                return false;
            };
            cursor = span_end;
            safe_start = cursor;
            continue;
        }

        if text.as_bytes()[cursor] == b'`' && !is_escaped(text, cursor) {
            push_range(safe, safe_start..cursor);
            let run_end = backtick_run_end(text, cursor, range.end);
            let run_length = run_end - cursor;
            let Some(span_end) = matching_backtick_end(text, run_end, range.end, run_length) else {
                return false;
            };
            cursor = span_end;
            safe_start = cursor;
            continue;
        }

        if text.as_bytes()[cursor] == b'[' && !is_escaped(text, cursor) {
            let Some((display, link_end)) = parse_inline_link(text, cursor, range.end) else {
                push_range(safe, safe_start..cursor);
                return false;
            };
            push_range(safe, safe_start..cursor);
            if !scan_safe_ranges(text, display, safe) {
                return false;
            }
            cursor = link_end;
            safe_start = cursor;
            continue;
        }

        if text.as_bytes()[cursor] == b']' && !is_escaped(text, cursor) {
            push_range(safe, safe_start..cursor);
            return false;
        }

        if let Some(url_end) = url_end(text, cursor, range.end) {
            push_range(safe, safe_start..cursor);
            cursor = url_end;
            safe_start = cursor;
            continue;
        }

        cursor += text[cursor..].chars().next().unwrap().len_utf8();
    }

    push_range(safe, safe_start..range.end);
    true
}

fn is_unsupported_line_construct(text: &str, position: usize) -> bool {
    let line_start = text[..position]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    if !text[line_start..position]
        .chars()
        .all(|character| matches!(character, ' ' | '\t'))
    {
        return false;
    }
    ["```", "~~~", ">>>", "...", ".. "]
        .iter()
        .any(|marker| text[position..].starts_with(marker))
}

fn parse_inline_link(text: &str, start: usize, end: usize) -> Option<(Range<usize>, usize)> {
    let mut cursor = start + 1;
    let mut close_bracket = None;
    while cursor < end {
        match text.as_bytes()[cursor] {
            b'\n' => return None,
            b'[' if !is_escaped(text, cursor) => return None,
            b']' if !is_escaped(text, cursor) => {
                close_bracket = Some(cursor);
                break;
            }
            b'\\' if cursor + 1 < end => cursor = advance_escaped_character(text, cursor, end),
            _ => cursor += text[cursor..].chars().next()?.len_utf8(),
        }
    }
    let close_bracket = close_bracket?;
    if text.as_bytes().get(close_bracket + 1) != Some(&b'(') {
        return None;
    }

    let mut depth = 1;
    cursor = close_bracket + 2;
    while cursor < end {
        match text.as_bytes()[cursor] {
            b'\n' => return None,
            b'\\' if cursor + 1 < end => cursor = advance_escaped_character(text, cursor, end),
            b'(' => {
                depth += 1;
                cursor += 1;
            }
            b')' => {
                depth -= 1;
                cursor += 1;
                if depth == 0 {
                    return Some((start + 1..close_bracket, cursor));
                }
            }
            _ => cursor += text[cursor..].chars().next()?.len_utf8(),
        }
    }
    None
}

fn url_end(text: &str, start: usize, end: usize) -> Option<usize> {
    url_prefix_length(text, start, end)?;

    let mut cursor = start;
    while cursor < end {
        let character = text[cursor..].chars().next()?;
        if character.is_whitespace() || matches!(character, '<' | '>' | '\'' | '"' | '`') {
            break;
        }
        cursor += character.len_utf8();
    }
    Some(cursor)
}

fn url_prefix_length(text: &str, start: usize, end: usize) -> Option<usize> {
    let rest = &text[start..end];
    uri_scheme_length(rest)
        .or_else(|| {
            rest.get(..7)
                .filter(|prefix| prefix.eq_ignore_ascii_case("mailto:"))
                .map(str::len)
        })
        .or_else(|| {
            rest.get(..4)
                .filter(|prefix| prefix.eq_ignore_ascii_case("www."))
                .map(str::len)
        })
}

fn angle_url_end(text: &str, start: usize, end: usize) -> Option<usize> {
    let mut cursor = start + 1;
    while cursor < end {
        let character = text[cursor..].chars().next()?;
        if character == '>' {
            return Some(cursor + 1);
        }
        if character.is_whitespace() || character == '<' {
            return None;
        }
        cursor += character.len_utf8();
    }
    None
}

fn uri_scheme_length(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    let mut cursor = 1;
    while cursor < bytes.len()
        && (bytes[cursor].is_ascii_alphanumeric() || matches!(bytes[cursor], b'+' | b'.' | b'-'))
    {
        cursor += 1;
    }
    (bytes.get(cursor..cursor + 3) == Some(b"://")).then_some(cursor + 3)
}

fn advance_escaped_character(text: &str, backslash: usize, end: usize) -> usize {
    let escaped = backslash + 1;
    if escaped >= end {
        return end;
    }
    escaped + text[escaped..].chars().next().unwrap().len_utf8()
}

fn backtick_run_end(text: &str, start: usize, end: usize) -> usize {
    let mut cursor = start;
    while cursor < end && text.as_bytes()[cursor] == b'`' {
        cursor += 1;
    }
    cursor
}

fn matching_backtick_end(
    text: &str,
    mut cursor: usize,
    end: usize,
    run_length: usize,
) -> Option<usize> {
    while cursor < end {
        if text.as_bytes()[cursor] == b'`' && !is_escaped(text, cursor) {
            let run_end = backtick_run_end(text, cursor, end);
            if run_end - cursor == run_length {
                return Some(run_end);
            }
            cursor = run_end;
        } else {
            cursor += text[cursor..].chars().next()?.len_utf8();
        }
    }
    None
}

fn is_escaped(text: &str, position: usize) -> bool {
    let backslashes = text[..position]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count();
    backslashes % 2 == 1
}

fn push_range(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    if range.start < range.end {
        ranges.push(range);
    }
}
