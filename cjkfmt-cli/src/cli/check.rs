use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use cjkfmt_parser::{Grammar, grammar_from_path};

use crate::{
    check::check_one_file, cli::utils::format_diagnostic, config::Config, document::Document,
};

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
        let mut document = Document::new(content, Grammar::Markdown, None::<String>);
        document.parse()?;
        let diagnostic = check_one_file(config, &document)?;
        diagnostics.extend(diagnostic);
    } else {
        for filename in filenames {
            let filename = filename.as_ref();
            let grammar = grammar_from_path(filename);
            let content = fs::read_to_string(filename)?;
            let mut document = Document::new(
                content,
                grammar,
                Some(filename.to_string_lossy().to_string()),
            );
            document.parse()?;
            let diagnostics_ = check_one_file(config, &document)?;
            diagnostics.extend(diagnostics_);
        }
    }
    for diagnostic in diagnostics {
        writeln!(stdout, "{}", format_diagnostic(&diagnostic))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use cjkfmt_core::position::Position;
    use tempfile::tempdir;

    use super::*;
    use crate::config::SpacingRule;

    #[test]
    fn check_command_routes_uppercase_python_and_reports_formatting_position() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("script.PY");
        fs::write(&path, "# 漢A\ndef f():\n    \"漢A\"\n    return \"漢A\"\n").unwrap();

        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;

        let mut output = Vec::new();
        check_command(&mut output, &config, &[&path]).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert_eq!(output.matches("W002").count(), 2);

        let mut document = Document::new(
            "# 漢A\ndef f():\n    \"漢A\"\n    return \"漢A\"\n",
            Grammar::Python,
            Some("script.PY"),
        );
        document.parse().unwrap();
        let diagnostics = check_one_file(&config, &document).unwrap();
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.start.clone())
                .collect::<Vec<_>>(),
            vec![Position::new(0, 3), Position::new(2, 6)]
        );
    }

    #[test]
    fn check_suppresses_w002_for_nonsemantic_mixed_literal_docstrings() {
        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        let source = concat!(
            "def f_string_first():\n",
            "    f\"X\" \"漢A\"\n",
            "def ordinary_first_f_string_second():\n",
            "    \"X\" f\"漢A\"\n",
            "def bytes_first():\n",
            "    b\"X\" \"漢A\"\n",
            "def ordinary_first_bytes_second():\n",
            "    \"X\" b\"漢A\"\n",
        );
        let mut document = Document::new(source, Grammar::Python, Some("mixed.py"));
        document.parse().unwrap();

        let diagnostics = check_one_file(&config, &document).unwrap();
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "W002")
        );
    }

    #[test]
    fn malformed_python_suppresses_w002_but_keeps_line_length_diagnostics() {
        let mut config = Config {
            max_width: 5,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        let mut document = Document::new("# 漢A\ndef broken(\n", Grammar::Python, Some("x.py"));
        document.parse().unwrap();

        let diagnostics = check_one_file(&config, &document).unwrap();
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "W002")
        );
    }

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
        check_command(&mut output, &config, &[&path]).unwrap();

        assert!(
            String::from_utf8(output).unwrap().contains("W002"),
            "uppercase .JSON should retain the Markdown grammar fallback"
        );
    }
}
