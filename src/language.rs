use std::path::Path;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// The user-facing kind of input document.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Language {
    Markdown,
    Json,
}

impl Language {
    /// Infer a language from the filename's extension.
    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        if extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown") {
            Some(Self::Markdown)
        } else if extension.eq_ignore_ascii_case("json") {
            Some(Self::Json)
        } else {
            None
        }
    }

    /// Resolve an explicit override or the canonical filename selection.
    pub(crate) fn explicit_or_inferred_path(language: Option<Self>, path: &Path) -> Option<Self> {
        language.or_else(|| Self::from_path(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_resolution_uses_the_explicit_language_before_the_filename() {
        assert_eq!(
            Language::explicit_or_inferred_path(
                Some(Language::Markdown),
                Path::new("document.json")
            ),
            Some(Language::Markdown)
        );
        assert_eq!(
            Language::explicit_or_inferred_path(None, Path::new("document.JSON")),
            Some(Language::Json)
        );
    }

    #[test]
    fn filename_inference_accepts_supported_extensions_case_insensitively() {
        let cases = [
            ("README.md", Some(Language::Markdown)),
            ("guide.markdown", Some(Language::Markdown)),
            ("README.MD", Some(Language::Markdown)),
            ("guide.MarkDown", Some(Language::Markdown)),
            ("config.json", Some(Language::Json)),
            ("config.JSON", Some(Language::Json)),
            ("config.JsOn", Some(Language::Json)),
        ];

        for (path, expected) in cases {
            assert_eq!(Language::from_path(Path::new(path)), expected, "{path}");
        }
    }

    #[test]
    fn language_resolution_returns_none_for_unrecognized_filenames() {
        for path in ["document.txt", "main.rs", "README", "README.md.txt"] {
            assert_eq!(Language::from_path(Path::new(path)), None, "{path}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn filename_inference_rejects_a_non_utf8_extension() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let path = OsString::from_vec(b"document.\xff".to_vec());
        assert_eq!(Language::from_path(Path::new(&path)), None);
    }
}
