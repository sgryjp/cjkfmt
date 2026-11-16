use std::ops::Range;

pub(crate) fn protected_prose_ranges(prose: &str) -> Vec<Range<usize>> {
    let lines = prose_lines(prose);
    let mut protected = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if let Some((marker, minimum_length)) = opening_fence(lines[index].1) {
            let start = lines[index].0.start;
            index += 1;
            while index < lines.len() && !closing_fence(lines[index].1, marker, minimum_length) {
                index += 1;
            }
            let end = if index < lines.len() {
                index += 1;
                lines[index - 1].0.end
            } else {
                prose.len()
            };
            protected.push(start..end);
            continue;
        }

        if let Some(block_indentation) = restructuredtext_directive_indentation(lines[index].1) {
            let start = lines[index].0.start;
            index = indented_block_end(&lines, index, block_indentation);
            let end = lines
                .get(index.saturating_sub(1))
                .map_or(prose.len(), |line| line.0.end);
            protected.push(start..end);
            continue;
        }

        if lines[index].1.trim_end().ends_with("::") {
            let start = lines[index].0.start;
            index = indented_block_end(&lines, index, indentation(lines[index].1));
            let end = lines
                .get(index.saturating_sub(1))
                .map_or(prose.len(), |line| line.0.end);
            protected.push(start..end);
            continue;
        }

        if lines[index].1.trim_start().starts_with(">>>") {
            let start = lines[index].0.start;
            index = doctest_end(&lines, index);
            let end = lines
                .get(index.saturating_sub(1))
                .map_or(prose.len(), |line| line.0.end);
            protected.push(start..end);
            continue;
        }

        index += 1;
    }

    protected
}

fn prose_lines(prose: &str) -> Vec<(Range<usize>, &str)> {
    let bytes = prose.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut cursor = 0;

    while cursor < bytes.len() {
        if matches!(bytes[cursor], b'\r' | b'\n') {
            let content_end = cursor;
            if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                cursor += 1;
            }
            cursor += 1;
            lines.push((start..cursor, &prose[start..content_end]));
            start = cursor;
        } else {
            cursor += 1;
        }
    }
    if start < prose.len() {
        lines.push((start..prose.len(), &prose[start..]));
    }
    lines
}

fn opening_fence(line: &str) -> Option<(char, usize)> {
    let indentation = indentation(line);
    let trimmed = &line[indentation..];
    let marker = trimmed.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (length >= 3).then_some((marker, length))
}

fn closing_fence(line: &str, marker: char, minimum_length: usize) -> bool {
    let indentation = indentation(line);
    let trimmed = &line[indentation..];
    let length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    length >= minimum_length
        && trimmed[length..]
            .chars()
            .all(|character| matches!(character, ' ' | '\t'))
}

fn restructuredtext_directive_indentation(line: &str) -> Option<usize> {
    let indentation = indentation(line);
    let directive = line[indentation..].strip_prefix(".. ")?;
    let separator = directive.find("::")?;
    let name = directive[..separator].trim_end();
    (!name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    .then_some(indentation)
}

fn indented_block_end(lines: &[(Range<usize>, &str)], start: usize, base_indent: usize) -> usize {
    let mut index = start + 1;
    while index < lines.len() && lines[index].1.trim().is_empty() {
        index += 1;
    }
    if index == lines.len() || indentation(lines[index].1) <= base_indent {
        return lines.len();
    }

    while index < lines.len() {
        if lines[index].1.trim().is_empty() {
            index += 1;
            continue;
        }
        if indentation(lines[index].1) <= base_indent {
            break;
        }
        index += 1;
    }
    index
}

fn doctest_end(lines: &[(Range<usize>, &str)], start: usize) -> usize {
    let mut index = start;
    while index < lines.len() {
        index += 1;
        if lines[index - 1].1.trim().is_empty() {
            break;
        }
    }
    index
}

fn indentation(line: &str) -> usize {
    line.bytes()
        .take_while(|byte| *byte == b' ' || *byte == b'\t')
        .count()
}
