use std::{
    fs,
    io::{Read, stdin},
    path::{Path, PathBuf},
};

use crate::{diagnostic::Diagnostic, line_break::LineBreaker};

pub fn check_command<W: std::io::Write>(
    stderr: &mut W,
    filenames: Vec<PathBuf>,
    max_width: usize,
) -> anyhow::Result<()> {
    // Read content of the specified files or standard input
    if filenames.is_empty() {
        let mut buf = String::with_capacity(1024);
        stdin().read_to_string(&mut buf)?;
        check_one_file(stderr, None, max_width, buf)?;
    } else {
        for filename in filenames {
            let content = fs::read_to_string(&filename)?;
            check_one_file(stderr, Some(&filename), max_width, content)?;
        }
    }
    Ok(())
}

fn check_one_file<W: std::io::Write>(
    stderr: &mut W,
    filename: Option<&Path>,
    max_width: usize,
    content: String,
) -> Result<(), anyhow::Error> {
    let breaker = LineBreaker::builder().max_width(max_width).build()?;

    for (line_no, line) in content.split_inclusive('\n').enumerate() {
        // TODO: Support LF only EOL code
        if let Some(line_break) = breaker.next_line_break(line) {
            let filename = filename.map(|p| p.to_string_lossy());
            let filename = filename.as_deref();

            let (precedings, _) = line.split_at(line_break);
            let column_no = precedings.encode_utf16().fold(0, |acc, _| acc + 1);
            let diagnostic = Diagnostic::new(
                filename,
                line_no,
                column_no,
                "W001".to_string(),
                format!("Line length exceeds {max_width} characters"),
            );
            writeln!(stderr, "{}", diagnostic)?;
        }
    }
    Ok(())
}
