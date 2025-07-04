use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    config::Config,
    core::{diagnostic::Diagnostic, position::Position},
    line_break::{BreakPoint, LineBreaker},
    spacing::search_possible_spacing_positions,
};

pub fn check_command<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    filenames: &[&Path],
) -> anyhow::Result<()> {
    let mut diagnostics = Vec::new();

    // Read content of the specified files or standard input
    if filenames.is_empty() {
        let mut content = String::with_capacity(1024);
        stdin().read_to_string(&mut content)?;
        let diagnostic = check_one_file(None, config.max_width, content)?;
        diagnostics.extend(diagnostic);
    } else {
        for filename in filenames {
            let content = fs::read_to_string(filename)?;
            let diagnostics_ =
                check_one_file(Some(&filename.to_string_lossy()), config.max_width, content)?;
            diagnostics.extend(diagnostics_);
        }
    }
    for diagnostic in diagnostics {
        writeln!(stdout, "{}", diagnostic)?;
    }
    Ok(())
}

pub(crate) fn check_one_file(
    filename: Option<&str>,
    max_width: u32,
    content: String,
) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let breaker = LineBreaker::builder().max_width(max_width).build()?;

    let mut diagnostics = Vec::new();
    for (line_index, line) in content.split_inclusive('\n').enumerate() {
        // TODO: Support CR only

        // Check line length problem
        if let Some(diagnostic) = check_line_length(&breaker, filename, line_index as u32, line) {
            diagnostics.push(diagnostic);
        }

        // Check spacing problems
        diagnostics.append(&mut check_spacing(filename, line_index as u32, line));
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

fn check_spacing(filename: Option<&str>, line_index: u32, line: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for i in search_possible_spacing_positions(line) {
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
