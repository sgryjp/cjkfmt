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
    use tempfile::tempdir;

    use super::*;
    use crate::config::SpacingRule;

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
