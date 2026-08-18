use std::path::Path;

/// Supported grammar types for parsing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grammar {
    Json,
    Markdown,
    MarkdownInline,
}

/// Infers the grammar type from the file extension of the given path.
///
/// Only an exact lowercase `.json` extension selects JSON. All other paths
/// retain the historical Markdown fallback used by the CLI commands.
pub fn grammar_from_path<P: AsRef<Path>>(path: P) -> Grammar {
    let path = path.as_ref();
    match path.extension().map(|s| s.to_str().unwrap()) {
        Some("json") => Grammar::Json,
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
    fn falls_back_to_markdown_for_all_other_paths() {
        for path in [
            "README.md",
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
