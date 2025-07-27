mod _log;
mod check;
mod cli;
mod config;
mod core;
mod format;
mod line_break;
mod spacing;

use std::io::stdout;

use anyhow::Context;
use clap::Parser;

use crate::{
    cli::{
        args::{self, CliArgs, ColorOutputMode},
        check::check_command,
        format::format_command,
    },
    config::Config,
};

fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    let config = Config::from_cli_args(&args).with_context(|| "failed to parse configuration")?;
    let mut stdout = stdout();

    // Control whether to colorize the output or not
    let condition = match args.color {
        ColorOutputMode::Always => yansi::Condition::ALWAYS,
        ColorOutputMode::Never => yansi::Condition::NEVER,
        ColorOutputMode::Auto => yansi::Condition::TTY_AND_COLOR,
    };
    yansi::whenever(condition);

    match args.command {
        args::Commands::Check { filenames } => {
            check_command(&mut stdout, &config, filenames.as_slice())?
        }
        args::Commands::Format { filenames } => {
            format_command(&mut stdout, &config, filenames.as_slice())?
        }
    }

    Ok(())
}

#[cfg(test)]
mod file_based_tests {
    use super::*;

    use regex::Regex;
    use serde::Deserialize;
    use serde_json::{self};
    use test_generator::test_resources;

    use crate::_log::test_log;
    use crate::check::check_one_file;
    use crate::core::diagnostic::Diagnostic;
    use crate::core::position::Position;
    use crate::format::format_one_file;

    #[derive(Default, Debug, Deserialize)]
    struct CheckTestCase {
        config: Config,
        input: String,
        diagnostics: Vec<Diagnostic>,
    }

    #[derive(Debug, Deserialize)]
    struct FormatTestCase {
        config: Config,
        input: String,
        output: String,
    }

    #[test_resources("test_cases/check/*.json")]
    fn check(resource: &str) {
        // Load the test case from the JSON file
        let content = std::fs::read_to_string(resource)
            .unwrap_or_else(|_| panic!("failed to read resource: {resource:?}"));
        let test_case: CheckTestCase = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("failed to parse resource: {resource:?}"));
        let actual = check_one_file(&test_case.config, Some(resource), &test_case.input)
            .unwrap_or_else(|_| panic!("failed on checking a file: {resource:?}"));

        // Find the offset of the original input text in the test data
        let matched_input = Regex::new(r##""input"\s*:\s*"(.*)""##)
            .unwrap()
            .captures(&content)
            .unwrap_or_else(|| panic!("`input` field not found in the test data file: {resource}"))
            .get(1)
            .unwrap();
        let lines_preceding: Vec<(usize, &str)> = content[..matched_input.start()]
            .match_indices("\n")
            .collect();
        let line_number_offset = lines_preceding.len();
        let (line_start_offset, _) = lines_preceding.last().expect("no preceding lines found");
        let column_number_offset = matched_input.start() - (line_start_offset + 1);

        // Print the diagnostics with adjusted line and column numbers
        for (i, diagnostic) in actual.iter().enumerate() {
            let diagnostic = Diagnostic::new(
                diagnostic.filename.clone(),
                Position {
                    line: diagnostic.start.line + line_number_offset as u32,
                    column: diagnostic.start.column + column_number_offset as u32,
                },
                Position {
                    line: diagnostic.end.line + line_number_offset as u32,
                    column: diagnostic.end.column + column_number_offset as u32,
                },
                diagnostic.code.clone(),
                diagnostic.message.clone(),
            );
            test_log!("diagnostics[{:2}] = {}", i, diagnostic);
        }
        assert_eq!(actual, test_case.diagnostics);
    }

    #[test_resources("test_cases/format/*.json")]
    fn format(resource: &str) {
        // Load the test case from the JSON file
        let content = std::fs::read_to_string(resource)
            .unwrap_or_else(|_| panic!("failed to read resource: {resource:?}"));
        let test_case: FormatTestCase = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("failed to parse resource: {resource:?}"));

        // Prepare a buffer to hold the formatted output
        let mut actual: Vec<u8> = Vec::with_capacity(1024);

        // Run the formatter on the input
        format_one_file(&mut actual, &test_case.config, &test_case.input)
            .unwrap_or_else(|_| panic!("failed on formatting a file: {resource:?}"));

        // Compare the actual output with the expected output
        assert_eq!(String::from_utf8_lossy(&actual), test_case.output);
    }
}
