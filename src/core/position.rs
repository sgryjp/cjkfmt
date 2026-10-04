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

    /// Converts a UTF-8 byte offset into a zero-based line and UTF-16 column.
    ///
    /// `offset` must be a valid character boundary in `content`.
    pub fn from_offset(content: &str, offset: usize) -> Self {
        let prefix = &content[..offset];
        let line = prefix.bytes().filter(|&byte| byte == b'\n').count() as u32;
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        let column = prefix[line_start..].encode_utf16().count() as u32;
        Self::new(line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_byte_offsets_to_utf16_positions_across_lines() {
        let content = "A😀B\n漢C";

        assert_eq!(Position::from_offset(content, 0), Position::new(0, 0));
        assert_eq!(
            Position::from_offset(content, "A😀".len()),
            Position::new(0, 3)
        );
        assert_eq!(
            Position::from_offset(content, "A😀B\n".len()),
            Position::new(1, 0)
        );
        assert_eq!(
            Position::from_offset(content, "A😀B\n漢".len()),
            Position::new(1, 1)
        );
    }
}
