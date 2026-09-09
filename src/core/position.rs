use serde::{Deserialize, Serialize};

/// A position in a text document.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Position {
    /// Zero-based line number of where the issue was detected.
    pub line: u32,

    /// Zero-based column number of where the issue was detected.
    ///
    /// This is the number of UTF-16 code units from the start of the line.
    pub column: u32,
}

impl Position {
    /// Creates a new `Position` with the specified `line` and `column` numbers.
    ///
    /// Both `line` and `column` are zero-based indices.
    /// The `column` represents the number of UTF-16 code units from the start of the line.
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}
