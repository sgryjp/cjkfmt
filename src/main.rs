mod args;
mod check;
mod diagnostic;
mod format;
mod line_break;

use std::io::{stderr, stdout};

use clap::Parser as _;

use crate::{args::Cli, check::check_command, format::format_command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut stdout = stdout();
    let mut stderr = stderr();

    // Control whether to colorize the output or not
    yansi::whenever(yansi::Condition::STDOUT_IS_TTY);

    match cli.command {
        args::Commands::Check {
            filenames,
            max_width,
        } => check_command(&mut stderr, filenames, max_width)?,
        args::Commands::Format {
            filenames,
            max_width,
        } => format_command(&mut stdout, filenames, max_width)?,
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    use std::path::PathBuf;

    #[test]
    fn check() -> anyhow::Result<()> {
        let mut stderr = Vec::new();
        let filenames = vec![PathBuf::from("sample_files/japanese.md")];
        let max_width = 80;

        yansi::whenever(yansi::Condition::NEVER);
        let result = check_command(&mut stderr, filenames, max_width);
        assert!(result.is_ok());
        let lines = String::from_utf8(stderr)?;
        let lines = lines
            .split('\n')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(
            lines[0],
            "sample_files/japanese.md:9:41: W001 Line length exceeds 80 characters"
        );
        assert_eq!(
            lines[1],
            "sample_files/japanese.md:19:41: W001 Line length exceeds 80 characters"
        );
        Ok(())
    }

    #[test]
    fn format() -> anyhow::Result<()> {
        let mut stdout = Vec::new();
        let filenames = vec![PathBuf::from("sample_files/japanese.md")];
        let max_width = 80;
        let expected_lines = std::fs::read_to_string("sample_files/japanese--max-width=80.md")?;
        let expected_lines = expected_lines
            .split('\n')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();

        yansi::whenever(yansi::Condition::NEVER);
        let result = format_command(&mut stdout, filenames, max_width);
        assert!(result.is_ok());
        let lines = String::from_utf8(stdout)?;
        let lines = lines
            .split('\n')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();
        assert_eq!(lines.len(), expected_lines.len());
        for i in 0..lines.len() {
            assert_eq!(lines[i], expected_lines[i]);
        }
        Ok(())
    }
}
