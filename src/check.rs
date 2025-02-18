use std::{
    fs,
    io::{stdin, Read},
    path::Path,
};

use yansi::Condition;

use crate::{diagnostic::Diagnostic, line_break::LineBreaker};

pub fn check_command(filename: Option<&Path>, max_width: usize) -> anyhow::Result<()> {
    // Control whether to colorize the output or not
    yansi::whenever(Condition::STDOUT_IS_TTY);

    // Read content of the specified file or standard input
    let content = match filename {
        Some(filename) => fs::read_to_string(filename)?,
        None => {
            let mut buf = String::with_capacity(1024);
            stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    // Read line by line and wrap them at the specified width
    let breaker = LineBreaker::builder().max_width(max_width).build()?;
    for (line_no, line) in content.split_inclusive('\n').enumerate() {
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
