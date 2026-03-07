//! The [`Document`] type for storing document content and metadata.

use cjkfmt_parser::{Grammar, errors::CjkfmtParseError, parse};
use tree_sitter::Tree;

/// Represents a document to be processed.
///
/// This struct holds the content of a file and its optional filename, which
/// is not available if the data to process was passed through the shell pipe (stdin).
#[derive(Debug, Clone)]
pub struct Document {
    /// The content of the document as a string.
    pub content: String,

    /// The grammar (programming language) of the document.
    pub grammar: Grammar,

    /// The name of the file from which the content was read, if available.
    /// This will be None if the content was read from stdin.
    pub filename: Option<String>,

    /// The syntax tree of the document.
    /// This is set after called [`parse`].
    tree: Option<Tree>,
}

impl Document {
    /// Creates a new document with the given content and filename.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the document.
    /// * `grammar` - The grammar (programming language) of the document.
    /// * `filename` - The optional name of the file from which the content was read.
    ///   None if the content was read from stdin.
    pub fn new<S1: Into<String>, S2: Into<String>>(
        content: S1,
        grammar: Grammar,
        filename: Option<S2>,
    ) -> Self {
        Self {
            content: content.into(),
            grammar,
            filename: filename.map(Into::into),
            tree: None,
        }
    }

    /// Parses the document.
    pub fn parse(&mut self) -> Result<(), CjkfmtParseError> {
        parse(self.grammar, &self.content).map(|tree| self.tree = Some(tree))
    }

    pub fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }
}
