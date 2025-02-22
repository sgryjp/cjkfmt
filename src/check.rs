use std::{
    fs,
    io::{Read, stdin},
    path::{Path, PathBuf},
};

use yansi::Condition;

use crate::{diagnostic::Diagnostic, line_break::LineBreaker};

pub fn check_command(filenames: Vec<PathBuf>, max_width: usize) -> anyhow::Result<()> {
    // Control whether to colorize the output or not
    yansi::whenever(Condition::STDOUT_IS_TTY);

    // Read content of the specified files or standard input
    if filenames.is_empty() {
        let mut buf = String::with_capacity(1024);
        stdin().read_to_string(&mut buf)?;
        check_one_file(None, max_width, buf)?;
    } else {
        for filename in filenames {
            let content = fs::read_to_string(&filename)?;
            check_one_file(Some(&filename), max_width, content)?;
        }
    }
    Ok(())
}

fn check_one_file(
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
                "Line is too long".to_string(),
            );
            eprintln!("{}", diagnostic);
        }
    }
    Ok(())
}
