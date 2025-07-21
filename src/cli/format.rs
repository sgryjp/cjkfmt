use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use crate::{config::Config, format::format_one_file};

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
