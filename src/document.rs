//! The [`Document`] type for storing document content and metadata.

use crate::language::Language;

/// Represents a document to be processed.
///
/// This struct holds the content of a file and its optional filename, which
/// is not available if the data to process was passed through the shell pipe (stdin).
#[derive(Debug, Clone)]
pub struct Document {
    /// The content of the document as a string.
    pub content: String,

    /// The selected language of the document, if known.
    pub language: Option<Language>,

    /// The name of the file from which the content was read, if available.
    /// This will be None if the content was read from stdin.
    pub filename: Option<String>,
}

impl Document {
    /// Creates a new document with the given content and filename.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the document.
    /// * `language` - The selected language, if known.
    /// * `filename` - The optional name of the file from which the content was read.
    ///   None if the content was read from stdin.
    pub fn new<S: Into<String>>(
        content: S,
        language: Option<Language>,
        filename: Option<String>,
    ) -> Self {
        Self {
            content: content.into(),
            language,
            filename,
        }
    }
}
