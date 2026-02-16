use thiserror::Error;

/// Error type for cjkfmt-parser crate.
#[derive(Error, Debug)]
pub enum CjkfmtParseError {
    /// The language was generated with an incompatible version of the Tree-sitter CLI.
    /// This means misconfiguration inside the source code.
    #[error("language version mismatch")]
    LanguageError(#[from] tree_sitter::LanguageError),

    #[error("parse error: {0}")]
    ParseError(String),
}
