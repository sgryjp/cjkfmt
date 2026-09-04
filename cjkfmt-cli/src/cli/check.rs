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
            let grammar = Language::grammar_or_inferred_path(language, filename);
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
    fn check_command_keeps_markdown_fallback_for_uppercase_json_files() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("document.JSON");
        fs::write(&path, "漢A\n").unwrap();

        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;

        let mut output = Vec::new();
        check_command(&mut output, &config, &[&path], None).unwrap();

        assert!(
            String::from_utf8(output).unwrap().contains("W002"),
            "uppercase .JSON should retain the Markdown grammar fallback"
        );
    }

    #[test]
    fn check_command_language_override_applies_markdown_to_every_file() {
        let directory = tempdir().unwrap();
        let json_path = directory.path().join("document.json");
        let markdown_path = directory.path().join("document.md");
        fs::write(&json_path, "漢A\n").unwrap();
        fs::write(&markdown_path, "漢A\n").unwrap();

        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        let mut output = Vec::new();
        check_command(
            &mut output,
            &config,
            &[&json_path, &markdown_path],
            Some(Language::Markdown),
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches("W002").count(), 2);
        assert!(output.contains(json_path.to_string_lossy().as_ref()));
        assert!(output.contains(markdown_path.to_string_lossy().as_ref()));
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
