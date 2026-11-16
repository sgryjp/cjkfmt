use std::ops::Range;

pub(crate) fn protected_comment_lines(source: &str, ranges: &[Range<usize>]) -> Vec<bool> {
    let lines = ranges
        .iter()
        .map(|range| comment_content(&source[range.clone()]))
        .collect::<Vec<_>>();
    let mut protected = vec![false; lines.len()];
    let mut index = 0;

    while index < lines.len() {
        if let Some((marker, length)) = opening_fence(lines[index]) {
            protected[index] = true;
            index += 1;
            while index < lines.len() {
                protected[index] = true;
                let closes = closing_fence(lines[index], marker, length);
                index += 1;
                if closes {
                    break;
                }
            }
            continue;
        }

        if let Some(indentation) = restructuredtext_directive_indentation(lines[index]) {
            index = protect_indented_block(&lines, &mut protected, index, indentation);
            continue;
        }

        if lines[index].trim_end().ends_with("::") {
            index =
                protect_indented_block(&lines, &mut protected, index, indentation(lines[index]));
            continue;
        }

        if lines[index].trim_start().starts_with(">>>") {
            index = protect_doctest(&lines, &mut protected, index);
            continue;
        }

        index += 1;
    }

    protected
}

fn comment_content(comment: &str) -> &str {
    let body = comment.strip_prefix('#').unwrap_or(comment);
    body.strip_prefix(' ')
        .unwrap_or(body)
        .trim_end_matches(['\r', '\n'])
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

fn protect_indented_block(
    lines: &[&str],
    protected: &mut [bool],
    directive_index: usize,
    directive_indentation: usize,
) -> usize {
    protected[directive_index] = true;
    let mut index = directive_index + 1;
    while index < lines.len() && lines[index].trim().is_empty() {
        protected[index] = true;
        index += 1;
    }
    if index == lines.len() || indentation(lines[index]) <= directive_indentation {
        protected[directive_index..].fill(true);
        return lines.len();
    }

    while index < lines.len() {
        if lines[index].trim().is_empty() {
            protected[index] = true;
            index += 1;
            continue;
        }
        if indentation(lines[index]) <= directive_indentation {
            break;
        }
        protected[index] = true;
        index += 1;
    }

    index
}

fn protect_doctest(lines: &[&str], protected: &mut [bool], start: usize) -> usize {
    let mut index = start;
    while index < lines.len() {
        protected[index] = true;
        index += 1;
        if lines[index - 1].trim().is_empty() {
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
