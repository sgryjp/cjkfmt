use std::{
    fs,
    io::{stdin, Read},
    path::PathBuf,
};

use crate::line_break::LineBreaker;

pub fn format_command(filename: Option<&PathBuf>, max_width: usize) -> anyhow::Result<()> {
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
    for line in content.split_inclusive('\n') {
        let mut substring = line;
        while let Some(line_break) = breaker.next_line_break(substring) {
            let (before, after) = substring.split_at(line_break);
            println!("{}", before);
            substring = after;
        }
        print!("{}", substring);
    }

    Ok(())
}
