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
