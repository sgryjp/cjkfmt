mod _log;
mod args;
mod check;
mod diagnostic;
mod format;
mod line_break;
mod position;
mod spacing;

use std::io::{stderr, stdout};

use clap::Parser as _;

use crate::{args::Cli, check::check_command, format::format_command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut stdout = stdout();
    let mut stderr = stderr();

    // Control whether to colorize the output or not
    yansi::whenever(yansi::Condition::STDOUT_IS_TTY);

    if cli.check {
        check_command(&mut stderr, cli.filenames, cli.max_width)?
    } else {
        format_command(&mut stdout, cli.filenames, cli.max_width)?
    }

    Ok(())
}

#[cfg(test)]
mod file_based_tests {
    use super::*;

    use std::path::PathBuf;

    use serde::Deserialize;
    use serde_json::{self};
    use test_generator::test_resources;

    use crate::_log::test_log;
    use crate::{check::check_one_file, diagnostic::Diagnostic};

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CheckTestCaseOptions {
        max_width: u32,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CheckTestCase {
        options: CheckTestCaseOptions,
        input: String,
        diagnostics: Vec<Diagnostic>,
    }

    #[test_resources("test_cases/check/*.json")]
    fn check(resource: &str) {
        let content = std::fs::read_to_string(resource)
            .unwrap_or_else(|_| panic!("failed to read resource: {:?}", resource));
        let test_case: CheckTestCase = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("failed to parse resource: {:?}", resource));
        let actual = check_one_file(Some(resource), test_case.options.max_width, test_case.input)
            .unwrap_or_else(|_| panic!("failed on checking a file: {:?}", resource));

        for (i, diagnostic) in actual.iter().enumerate() {
            test_log!("diagnostics[{:2}] = {}", i, diagnostic);
        }
        assert_eq!(actual, test_case.diagnostics);
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
