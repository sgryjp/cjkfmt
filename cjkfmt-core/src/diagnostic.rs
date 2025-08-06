//! The [`Diagnostic`] type for storing diagnostic information.
use serde::{Deserialize, Serialize};

use crate::position::Position;

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
