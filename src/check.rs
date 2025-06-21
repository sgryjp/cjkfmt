use std::{
    fs,
    io::{Read, stdin},
    path::PathBuf,
};

use unicode_segmentation::UnicodeSegmentation;

use crate::{
    diagnostic::Diagnostic,
    line_break::{BreakPoint, LineBreaker},
    position::Position,
};

pub fn check_command<W: std::io::Write>(
    stderr: &mut W,
    filenames: Vec<PathBuf>,
    max_width: u32,
) -> anyhow::Result<()> {
    let mut diagnostics = Vec::new();

    // Read content of the specified files or standard input
    if filenames.is_empty() {
        let mut buf = String::with_capacity(1024);
        stdin().read_to_string(&mut buf)?;
        let diagnostic = check_one_file(None, max_width, buf)?;
        diagnostics.extend(diagnostic);
    } else {
        for filename in filenames {
            let content = fs::read_to_string(&filename)?;
            let diagnostics_ = check_one_file(
                Some(&filename.as_path().to_string_lossy()),
                max_width,
                content,
            )?;
            diagnostics.extend(diagnostics_);
        }
    }
    for diagnostic in diagnostics {
        writeln!(stderr, "{}", diagnostic)?;
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
    for (line_no, line) in content.split_inclusive('\n').enumerate() {
        // TODO: Support CR only

        // Check line length problem
        if let Some(diagnostic) = check_line_length(&breaker, filename, line_no as u32, line) {
            diagnostics.push(diagnostic);
        }
    }
    Ok(diagnostics)
}

fn check_line_length(
    breaker: &LineBreaker,
    filename: Option<&str>,
    line_no: u32,
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
    let column_no = precedings.encode_utf16().fold(0u32, |acc, _| acc + 1);
    let start = Position::new(line_no, column_no);
    let next_char_len = followings
        .graphemes(true)
        .next()
        .map(|s| s.encode_utf16().fold(0u32, |acc, _| acc + 1))
        .unwrap_or(0u32);
    let end = Position::new(line_no, column_no + next_char_len);
    Some(Diagnostic::new(
        filename,
        start,
        end,
        "W001".to_string(),
        format!("Line length exceeds {} characters", breaker.max_width()),
    ))
}
