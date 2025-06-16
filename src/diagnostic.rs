//! The [`Diagnostic`] type for storing diagnostic information.
use core::fmt::Display;

use serde::Serialize;
use yansi::Paint;

/// Diagnostic information for a single issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// The name of the file where the issue was detected.
    filename: Option<String>,

    /// Zero-based line number of where the issue was detected.
    line: u32,

    /// Zero-based column number of where the issue was detected.
    ///
    /// This is the number of UTF-16 code units from the start of the line.
    column: u32,

    /// A unique code identifying the issue.
    code: String,

    /// A human-readable message describing the issue.
    message: String,
}

impl Diagnostic {
    pub fn new<S: Into<String>>(
        filename: Option<S>,
        line: u32,
        column: u32,
        code: String,
        message: String,
    ) -> Self {
        Self {
            filename: filename.map(Into::into),
            line,
            column,
            code,
            message,
        }
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let filename = self.filename.as_deref().unwrap_or("<stdin>");
        let buf = format!(
            "{}\0{}\0{}\0{}\0{}",
            filename,
            self.line + 1,
            self.column + 1,
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
            "{}{colon}{}{colon}{}{colon} {} {}",
            filename, line, column, code, message
        )
    }
}
