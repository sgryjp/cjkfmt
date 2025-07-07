//! The [`Diagnostic`] type for storing diagnostic information.
use core::fmt::Display;

use serde::{Deserialize, Serialize};
use yansi::Paint;

use crate::core::position::Position;

/// Diagnostic information for a single issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// The name of the file where the issue was detected.
    pub filename: Option<String>,

    /// The start position of the half-open range (inclusive) where the issue applies.
    pub start: Position,

    /// The end position of the half-open range (exclusive) where the issue applies.
    pub end: Position,

    /// A unique code identifying the issue.
    pub code: String,

    /// A human-readable message describing the issue.
    pub message: String,
}

impl Diagnostic {
    pub fn new<S: Into<String>>(
        filename: Option<S>,
        start: Position,
        end: Position,
        code: String,
        message: String,
    ) -> Self {
        Self {
            filename: filename.map(Into::into),
            start,
            end,
            code,
            message,
        }
    }
}

impl Display for Diagnostic {
    /// Formats the diagnostic information into a human-readable string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let filename = self.filename.as_deref().unwrap_or("<stdin>");
        let line = self.start.line;
        let column = self.start.column;
        let buf = format!(
            "{}\0{}\0{}\0{}\0{}",
            filename,
            line + 1,
            column + 1,
            self.code,
            self.message
        );
        let tokens: Vec<&str> = buf.split('\0').collect();
        let colon = ":".cyan();
        let filename = tokens[0].white().bold();
        let line = tokens[1];
        let column = tokens[2];
        let code = tokens[3].yellow();
        let message = tokens[4];
        write!(
            f,
            "{filename}{colon}{line}{colon}{column}{colon} {code} {message}"
        )
    }
}
