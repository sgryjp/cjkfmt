use std::path::Path;

/// Supported grammar types for parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    Json,
    Markdown,
    MarkdownInline,
}

/// Grammar types that can be selected from a named file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileGrammar {
    Json,
    Markdown,
}

impl From<FileGrammar> for Grammar {
    fn from(grammar: FileGrammar) -> Self {
        match grammar {
            FileGrammar::Json => Grammar::Json,
            FileGrammar::Markdown => Grammar::Markdown,
        }
    }
}

/// Infers a filename-selectable grammar from the file extension of `path`.
///
/// Matching is ASCII-case-insensitive and only Markdown and JSON extensions
/// are recognized. A filename without a UTF-8 extension is not selectable.
pub fn grammar_from_path(path: &Path) -> Option<FileGrammar> {
    let extension = path.extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown") {
        Some(FileGrammar::Markdown)
    } else if extension.eq_ignore_ascii_case("json") {
        Some(FileGrammar::Json)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_the_canonical_file_grammar_for_supported_extensions() {
        let cases = [
            ("README.md", Some(FileGrammar::Markdown)),
            ("guide.markdown", Some(FileGrammar::Markdown)),
            ("README.MD", Some(FileGrammar::Markdown)),
            ("guide.MarkDown", Some(FileGrammar::Markdown)),
            ("config.json", Some(FileGrammar::Json)),
            ("config.JSON", Some(FileGrammar::Json)),
            ("config.JsOn", Some(FileGrammar::Json)),
            ("notes.txt", None),
            ("main.rs", None),
            ("README", None),
            ("README.md.txt", None),
        ];

        for (path, expected) in cases {
            assert_eq!(grammar_from_path(Path::new(path)), expected, "{path}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_non_utf8_extension() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let path = OsString::from_vec(b"document.\xff".to_vec());
        assert_eq!(grammar_from_path(Path::new(&path)), None);
    }
}
