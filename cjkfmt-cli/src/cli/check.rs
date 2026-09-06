use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use super::args::Language;

use crate::{
    check::check_one_file, cli::utils::format_diagnostic, config::Config, document::Document,
};

pub fn check_command<W, P>(
    stdout: &mut W,
    config: &Config,
    filenames: &[P],
    language: Option<Language>,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
{
    let mut stdin = stdin();
    check_command_with_reader(stdout, config, filenames, language, &mut stdin)
}

fn check_command_with_reader<W, P, R>(
    stdout: &mut W,
    config: &Config,
    filenames: &[P],
    language: Option<Language>,
    stdin: &mut R,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
    R: Read,
{
    let mut diagnostics = Vec::new();

    if filenames.is_empty() {
        let mut content = String::with_capacity(1024);
        stdin.read_to_string(&mut content)?;
        let grammar = Language::grammar_or_markdown_default(language);
        let document = Document::new(content, grammar, None::<String>);
        diagnostics.extend(check_one_file(config, &document)?);
    } else {
        for filename in filenames {
            let filename = filename.as_ref();
            let file_grammar =
                Language::grammar_or_inferred_path(language, filename).ok_or_else(|| {
                    anyhow::anyhow!(
                        "could not infer the language for {}; specify it with --language",
                        filename.display()
                    )
                })?;
            let grammar: cjkfmt_parser::Grammar = file_grammar.into();
            let content = fs::read_to_string(filename)?;
            let document = Document::new(
                content,
                grammar,
                Some(filename.to_string_lossy().to_string()),
            );
            diagnostics.extend(check_one_file(config, &document)?);
        }
    }
    for diagnostic in diagnostics {
        writeln!(stdout, "{}", format_diagnostic(&diagnostic))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;
    use crate::{cli::args::Language, config::SpacingRule};

    #[test]
    fn check_command_selects_json_for_an_uppercase_json_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("document.JSON");
        fs::write(&path, "{\"value\":\"漢A\"}\n").unwrap();

        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;

        let mut output = Vec::new();
        check_command(&mut output, &config, &[&path], None).unwrap();

        assert!(!String::from_utf8(output).unwrap().contains("W002"));
    }

    #[test]
    fn check_command_selects_markdown_for_mixed_case_markdown_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("document.MarkDown");
        fs::write(&path, "漢A\n").unwrap();

        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;

        let mut output = Vec::new();
        check_command(&mut output, &config, &[&path], None).unwrap();

        assert!(String::from_utf8(output).unwrap().contains("W002"));
    }

    #[test]
    fn check_command_rejects_unrecognized_named_files_without_an_override() {
        let directory = tempdir().unwrap();
        for filename in ["document.txt", "document"] {
            let path = directory.path().join(filename);
            fs::write(&path, "漢A\n").unwrap();

            let error = check_command(&mut Vec::new(), &Config::default(), &[&path], None)
                .expect_err("unsupported named files should require an explicit language");
            let message = error.to_string();
            assert!(message.contains(path.to_string_lossy().as_ref()));
            assert!(message.contains("specify it with --language"));
        }
    }

    #[test]
    fn check_command_language_override_applies_markdown_to_every_file() {
        let directory = tempdir().unwrap();
        let json_path = directory.path().join("document.json");
        let markdown_path = directory.path().join("document.md");
        let unknown_path = directory.path().join("document.txt");
        fs::write(&json_path, "漢A\n").unwrap();
        fs::write(&markdown_path, "漢A\n").unwrap();
        fs::write(&unknown_path, "漢A\n").unwrap();

        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        let mut output = Vec::new();
        check_command(
            &mut output,
            &config,
            &[&json_path, &markdown_path, &unknown_path],
            Some(Language::Markdown),
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches("W002").count(), 3);
        assert!(output.contains(json_path.to_string_lossy().as_ref()));
        assert!(output.contains(markdown_path.to_string_lossy().as_ref()));
        assert!(output.contains(unknown_path.to_string_lossy().as_ref()));
    }

    #[test]
    fn check_command_defaults_stdin_to_markdown() {
        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        let mut input = "漢A\n".as_bytes();
        let mut output = Vec::new();

        check_command_with_reader(&mut output, &config, &[] as &[PathBuf], None, &mut input)
            .unwrap();

        assert!(String::from_utf8(output).unwrap().contains("W002"));
    }

    #[test]
    fn check_command_language_json_overrides_markdown_path_without_spacing_diagnostic() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("document.md");
        fs::write(&path, "{\"value\":\"漢A\"}\n").unwrap();

        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        let mut output = Vec::new();
        check_command(&mut output, &config, &[&path], Some(Language::Json)).unwrap();

        assert!(!String::from_utf8(output).unwrap().contains("W002"));
    }

    #[test]
    fn check_command_language_json_suppresses_spacing_but_keeps_line_length() {
        let mut config = Config {
            max_width: 5,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        let mut input = "{\"value\":\"漢A\"}\n".as_bytes();
        let mut output = Vec::new();
        check_command_with_reader(
            &mut output,
            &config,
            &[] as &[PathBuf],
            Some(Language::Json),
            &mut input,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("W001"));
        assert!(!output.contains("W002"));
    }
}
