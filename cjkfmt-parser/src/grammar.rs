use std::path::Path;

/// Supported grammar types for parsing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grammar {
    Json,
    Python,
    Markdown,
    MarkdownInline,
}

/// Infers the grammar type from the file extension of the given path.
///
/// `.json` selects JSON and `.py` selects Python case-insensitively. All other
/// paths retain the historical Markdown fallback used by the CLI commands.
pub fn grammar_from_path<P: AsRef<Path>>(path: P) -> Grammar {
    let path = path.as_ref();
    match path.extension().map(|s| s.to_str().unwrap()) {
        Some("json") => Grammar::Json,
        Some(extension) if extension.eq_ignore_ascii_case("py") => Grammar::Python,
        _ => Grammar::Markdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_json_only_for_an_exact_lowercase_extension() {
        assert_eq!(grammar_from_path("config.json"), Grammar::Json);
        assert_eq!(grammar_from_path("config.JSON"), Grammar::Markdown);
    }

    #[test]
    fn selects_python_case_insensitively() {
        assert_eq!(grammar_from_path("script.py"), Grammar::Python);
        assert_eq!(grammar_from_path("script.PY"), Grammar::Python);
        assert_eq!(grammar_from_path("script.pyw"), Grammar::Markdown);
    }

    #[test]
    fn falls_back_to_markdown_for_all_other_paths() {
        for path in [
            "README.md",
            "script.pyw",
            "guide.markdown",
            "README.MD",
            "guide.MarkDown",
            "notes.txt",
            "main.rs",
            "README",
            "README.md.txt",
        ] {
            assert_eq!(grammar_from_path(path), Grammar::Markdown, "{path}");
        }
    }
}
