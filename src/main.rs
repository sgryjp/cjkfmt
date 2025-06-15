mod _log;
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

    if cli.check {
        check_command(&mut stderr, cli.filenames, cli.max_width)?
    } else {
        format_command(&mut stdout, cli.filenames, cli.max_width)?
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use serde::Deserialize;
    use serde_json::{self, Value};
    use test_generator::test_resources;

    use crate::check::check_one_file;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CheckTestCaseOptions {
        max_width: u16,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CheckTestCase {
        options: CheckTestCaseOptions,
        input: String,
        expected: Value,
    }

    #[test_resources("test_cases/check/*.json")]
    fn check(resource: &str) {
        let content = std::fs::read_to_string(resource)
            .expect(format!("failed to read resource: {:?}", resource).as_str());
        let test_case: CheckTestCase = serde_json::from_str(&content)
            .expect(format!("failed to parse resource: {:?}", resource).as_str());
        let diagnostics = check_one_file(
            Some(resource),
            test_case.options.max_width as usize,
            test_case.input,
        )
        .expect(format!("failed on checking a file: {:?}", resource).as_str());

        let actual: serde_json::Value = serde_json::to_value(diagnostics)
            .expect(format!("failed to serialize actual result: {:?}", resource).as_str());
        assert_eq!(actual, test_case.expected);
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
