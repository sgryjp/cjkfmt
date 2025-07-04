mod _log;
mod args;
mod check;
mod config;
mod core;
mod format;
mod line_break;
mod spacing;

use std::io::stdout;

use anyhow::Context;
use clap::Parser;

use crate::{args::Cli, check::check_command, config::Config, format::format_command};

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let config = Config::from_cli_args(&args).with_context(|| "failed to parse configuration")?;
    let mut stdout = stdout();

    // Control whether to colorize the output or not
    let condition = match args.color {
        args::ColorOutputMode::Always => yansi::Condition::ALWAYS,
        args::ColorOutputMode::Never => yansi::Condition::NEVER,
        args::ColorOutputMode::Auto => yansi::Condition::TTY_AND_COLOR,
    };
    yansi::whenever(condition);

    if args.check {
        check_command(&mut stdout, &config, &args.filenames())?
    } else {
        format_command(&mut stdout, &config, &args.filenames())?
    }

    Ok(())
}

#[cfg(test)]
mod file_based_tests {
    use super::*;

    use serde::Deserialize;
    use serde_json::{self};
    use test_generator::test_resources;

    use crate::_log::test_log;
    use crate::check::check_one_file;
    use crate::core::diagnostic::Diagnostic;
    use crate::format::format_one_file;

    #[derive(Debug, Deserialize)]
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
        let content = std::fs::read_to_string(resource)
            .unwrap_or_else(|_| panic!("failed to read resource: {:?}", resource));
        let test_case: CheckTestCase = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("failed to parse resource: {:?}", resource));
        let actual = check_one_file(Some(resource), test_case.config.max_width, &test_case.input)
            .unwrap_or_else(|_| panic!("failed on checking a file: {:?}", resource));

        for (i, diagnostic) in actual.iter().enumerate() {
            test_log!("diagnostics[{:2}] = {}", i, diagnostic);
        }
        assert_eq!(actual, test_case.diagnostics);
    }

    #[test_resources("test_cases/format/*.json")]
    fn format(resource: &str) {
        let content = std::fs::read_to_string(resource)
            .unwrap_or_else(|_| panic!("failed to read resource: {:?}", resource));
        let test_case: FormatTestCase = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("failed to parse resource: {:?}", resource));
        let mut actual: Vec<u8> = Vec::with_capacity(1024);
        format_one_file(&mut actual, &test_case.config, &test_case.input)
            .unwrap_or_else(|_| panic!("failed on checking a file: {:?}", resource));

        assert_eq!(String::from_utf8_lossy(&actual), test_case.output);
    }
}
