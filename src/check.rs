use std::{
    fs,
    io::{Read, stdin},
    path::PathBuf,
};

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
        let line_break = match breaker.next_line_break(line) {
            BreakPoint::WrapPoint(i) => i,
            BreakPoint::EndOfLine(_) | BreakPoint::EndOfText(_) => continue,
        };

        let (precedings, _) = line.split_at(line_break);
        let column_no = precedings.encode_utf16().fold(0u32, |acc, _| acc + 1);
        let start = Position::new(line_no as u32, column_no);
        let end = start.clone(); // TODO:
        let diagnostic = Diagnostic::new(
            filename,
            start,
            end,
            "W001".to_string(),
            format!("Line length exceeds {max_width} characters"),
        );
        diagnostics.push(diagnostic);
    }
    Ok(diagnostics)
}
