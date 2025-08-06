use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use crate::{check::check_one_file, config::Config};

pub fn check_command<W, P>(stdout: &mut W, config: &Config, filenames: &[P]) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
{
    let mut diagnostics = Vec::new();

    // Read content of the specified files or standard input
    if filenames.is_empty() {
        let mut content = String::with_capacity(1024);
        stdin().read_to_string(&mut content)?;
        let diagnostic = check_one_file(config, None, &content)?;
        diagnostics.extend(diagnostic);
    } else {
        for filename in filenames {
            let filename = filename.as_ref();
            let content = fs::read_to_string(filename)?;
            let diagnostics_ = check_one_file(config, Some(&filename.to_string_lossy()), &content)?;
            diagnostics.extend(diagnostics_);
        }
    }
    for diagnostic in diagnostics {
        writeln!(stdout, "{diagnostic}")?;
    }
    Ok(())
}
