//! The [`Document`] type for storing document content and metadata.

/// Represents a document to be processed.
///
/// This struct holds the content of a file and its optional filename, which
/// is not available if the data to process was passed through the shell pipe (stdin).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The content of the document as a string.
    pub content: String,

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
    /// * `filename` - The optional name of the file from which the content was read.
    ///                None if the content was read from stdin.
    pub fn new<S1: Into<String>, S2: Into<String>>(content: S1, filename: Option<S2>) -> Self {
        Self {
            content: content.into(),
            filename: filename.map(Into::into),
        }
    }
}
