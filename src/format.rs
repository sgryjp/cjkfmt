use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use crate::{
    config::Config,
    core::lines_inclusive::LinesInclusiveExt,
    line_break::{BreakPoint, LineBreaker},
};

pub fn format_command<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    filenames: &[&Path],
) -> anyhow::Result<()> {
    // Read content of the specified files or standard input
    if filenames.is_empty() {
        let mut content = String::with_capacity(1024);
        stdin().read_to_string(&mut content)?;
        format_one_file(stdout, config, &content)?;
    } else {
        for filename in filenames.iter() {
            let content = fs::read_to_string(filename)?;
            format_one_file(stdout, config, &content)?;
        }
    }
    Ok(())
}

pub(crate) fn format_one_file<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    content: &str,
) -> Result<(), anyhow::Error> {
    let line_breaker = LineBreaker::builder().max_width(config.max_width).build()?;

    // Iterate over each line in the input content, including line endings
    for line in content.lines_inclusive() {
        let mut remainings = line;

        // Iterate over wrap points in the line
        while let BreakPoint::WrapPoint {
            overflow_pos,
            adjustment,
        } = line_breaker.next_line_break(remainings)
        {
            // Write the part before the wrap point
            let (before, after) = remainings.split_at(overflow_pos - adjustment);
            writeln!(stdout, "{before}")?;
            remainings = after;
        }

        // Write any remaining part of the line after the last wrap point
        write!(stdout, "{remainings}")?;
    }
    Ok(())
}
