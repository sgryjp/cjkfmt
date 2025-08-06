use cjkfmt_core::{diagnostic::Diagnostic, lines_inclusive::LinesInclusiveExt, position::Position};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    config::Config,
    line_break::{BreakPoint, LineBreaker},
    spacing::search_possible_spacing_positions,
};

pub(crate) fn check_one_file(
    config: &Config,
    filename: Option<&str>,
    content: &str,
) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let breaker = LineBreaker::builder()
        .ambiguous_width(config.ambiguous_width)
        .max_width(config.max_width)
        .build()?;

    let mut diagnostics = Vec::new();
    for (line_index, line) in content.lines_inclusive().enumerate() {
        // Check line length problem
        if let Some(diagnostic) = check_line_length(&breaker, filename, line_index as u32, line) {
            diagnostics.push(diagnostic);
        }

        // Check spacing problems
        diagnostics.append(&mut check_spacing(
            config,
            filename,
            line_index as u32,
            line,
        ));
    }
    Ok(diagnostics)
}

fn check_line_length(
    breaker: &LineBreaker,
    filename: Option<&str>,
    line_index: u32,
    line: &str,
) -> Option<Diagnostic> {
    let overflow_pos = match breaker.next_line_break(line) {
        BreakPoint::WrapPoint {
            overflow_pos,
            adjustment: _,
        } => overflow_pos,
        BreakPoint::EndOfLine(_) | BreakPoint::EndOfText(_) => {
            return None;
        }
    };
    let (precedings, followings) = line.split_at(overflow_pos);
    let column_index = precedings.encode_utf16().fold(0u32, |acc, _| acc + 1);
    let start = Position::new(line_index, column_index);
    let next_char_len = followings
        .graphemes(true)
        .next()
        .map(|s| s.encode_utf16().fold(0u32, |acc, _| acc + 1))
        .unwrap_or(0u32);
    let end = Position::new(line_index, column_index + next_char_len);
    Some(Diagnostic::new(
        filename,
        start,
        end,
        "W001".to_string(),
        format!("Line length exceeds {} characters", breaker.max_width()),
    ))
}

fn check_spacing(
    config: &Config,
    filename: Option<&str>,
    line_index: u32,
    line: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for i in search_possible_spacing_positions(config, line) {
        // Calculate number of characters from the beginning of the line
        let column_index = line[..i].graphemes(true).fold(0u32, |acc, s| {
            acc + s.encode_utf16().fold(0u32, |acc, _| acc + 1)
        });

        // Get number of Unicode scalar values of the next character
        let next_char_len = line
            .graphemes(true)
            .nth(i)
            .map(|s| s.encode_utf16().fold(0u32, |acc, _| acc + 1))
            .unwrap_or(0u32);

        diagnostics.push(Diagnostic::new(
            filename,
            Position::new(line_index, column_index),
            Position::new(line_index, column_index + next_char_len),
            "W002".to_string(),
            "Possible spacing position found".to_string(),
        ));
    }
    diagnostics
}
