mod _log;
mod check;
mod cli;
mod config;
mod core;
mod document;
mod format;
mod formatting;
mod language;
mod line_break;
mod markdown_prose;
mod parser;
mod spacing;
mod spacing_checker;

#[cfg(test)]
mod check_test_case;

use std::io::stdout;

use anyhow::Context;
use clap::Parser;

use crate::{
    cli::{
        args::{self, CliArgs, ColorOutputMode},
        check::check_command,
        debug_cst::debug_cst_command,
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
        args::Commands::Check {
            language,
            filenames,
        } => check_command(&mut stdout, &config, filenames.as_slice(), language)?,
        args::Commands::DebugCst {
            language,
            filenames,
        } => debug_cst_command(&mut stdout, filenames.as_slice(), language)?,
        args::Commands::Format {
            write,
            language,
            filenames,
        } => format_command(&mut stdout, &config, filenames.as_slice(), write, language)?,
    }

    Ok(())
}

#[cfg(test)]
mod file_based_tests {
    use std::path::Path;

    use crate::core::diagnostic::Diagnostic;
    use test_generator::test_resources;

    use crate::_log::test_log;
    use crate::check::check_one_file;
    use crate::check_test_case::load_check_test_case;
    use crate::cli::utils::format_diagnostic;

    #[test_resources("test_cases/check/*/case.json")]
    fn check(resource: &str) {
        let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let test_case = load_check_test_case(Path::new(resource), repository_root)
            .unwrap_or_else(|error| panic!("{error:#}"));
        let input_path = repository_root.join(
            test_case
                .document
                .filename
                .as_deref()
                .expect("file-based test documents have filenames"),
        );
        let input_bytes_before = std::fs::read(&input_path).unwrap_or_else(|error| {
            panic!("failed to read input {}: {error}", input_path.display())
        });
        assert_eq!(input_bytes_before, test_case.document.content.as_bytes());

        let actual = check_one_file(&test_case.config, &test_case.document)
            .unwrap_or_else(|error| panic!("failed to check case {resource:?}: {error:#}"));

        assert_eq!(
            std::fs::read(&input_path).expect("check tests should not remove their input files"),
            input_bytes_before,
            "check tests must not modify input files"
        );

        for (i, diagnostic) in actual.iter().enumerate() {
            let formatted = format_diagnostic(diagnostic);
            test_log!("diagnostics[{i:2}] = {formatted}");
        }
        assert_eq!(
            actual.len(),
            test_case.diagnostics.len(),
            "diagnostic count mismatch for {resource}"
        );
        actual
            .iter()
            .zip(&test_case.diagnostics)
            .for_each(|(actual, expected)| assert_diagnostics_are_equal(actual, expected));
    }

    fn assert_diagnostics_are_equal(a: &Diagnostic, b: &Diagnostic) {
        match (&a.filename, &b.filename) {
            (Some(f1), Some(f2)) => {
                // Check whether the longer one ends with the shorter one
                // so that the difference of working directory are ignored.
                if f1.len() < f2.len() {
                    assert!(
                        f2.ends_with(f1),
                        "filename does not match: {f1:?} and {f2:?}"
                    );
                } else {
                    assert!(
                        f1.ends_with(f2),
                        "filename does not match: {f1:?} and {f2:?}"
                    );
                }
            }
            (f1, f2) => assert_eq!(f1, f2),
        };
        assert_eq!(a.start, b.start);
        assert_eq!(a.end, b.end);
        assert_eq!(a.code, b.code);
        assert_eq!(a.message, b.message);
    }
}
