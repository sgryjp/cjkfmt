use std::{
    fs,
    io::{Read, stdin},
    path::PathBuf,
};

use crate::line_break::{BreakPoint, LineBreaker};

pub fn format_command<W: std::io::Write>(
    stdout: &mut W,
    filenames: Vec<PathBuf>,
    max_width: usize,
) -> anyhow::Result<()> {
    // Read content of the specified files or standard input
    if filenames.is_empty() {
        let mut buf = String::with_capacity(1024);
        stdin().read_to_string(&mut buf)?;
        format_one_file(stdout, max_width, buf)?;
    } else {
        for filename in filenames.iter() {
            let content = fs::read_to_string(filename)?;
            format_one_file(stdout, max_width, content)?;
        }
    }
    Ok(())
}

fn format_one_file<W: std::io::Write>(
    stdout: &mut W,
    max_width: usize,
    content: String,
) -> Result<(), anyhow::Error> {
    let breaker = LineBreaker::builder().max_width(max_width).build()?;
    for line in content.split_inclusive('\n') {
        // TODO: Support LF only EOL code
        let mut substring = line;
        loop {
            let line_break = match breaker.next_line_break(substring) {
                BreakPoint::WrapPoint(i) => i,
                BreakPoint::EndOfLine(_) | BreakPoint::EndOfText(_) => break,
            };

            let (before, after) = substring.split_at(line_break);
            writeln!(stdout, "{}", before)?;
            substring = after;
        }
        writeln!(stdout, "{}", substring)?;
    }
    Ok(())
}
