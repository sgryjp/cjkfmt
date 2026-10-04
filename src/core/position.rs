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
    /// `offset` must be a valid character boundary in `content`. An offset
    /// between the CR and LF of a CRLF terminator maps to the preceding line's
    /// end.
    pub fn from_offset(content: &str, offset: usize) -> Self {
        let bytes = content.as_bytes();
        let mut line = 0;
        let mut line_start = 0;
        let mut index = 0;

        while index < offset {
            match bytes[index] {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    if index + 1 == offset {
                        let column = content[line_start..index].encode_utf16().count() as u32;
                        return Self::new(line, column);
                    }
                    line += 1;
                    index += 2;
                    line_start = index;
                }
                b'\r' | b'\n' => {
                    line += 1;
                    index += 1;
                    line_start = index;
                }
                _ => index += 1,
            }
        }

        let column = content[line_start..offset].encode_utf16().count() as u32;
        Self::new(line, column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_byte_offsets_to_utf16_positions_for_each_line_ending() {
        for line_ending in ["\n", "\r\n", "\r"] {
            let content = format!("A😀B{line_ending}漢C");

            assert_eq!(Position::from_offset(&content, 0), Position::new(0, 0));
            assert_eq!(
                Position::from_offset(&content, "A😀".len()),
                Position::new(0, 3)
            );
            assert_eq!(
                Position::from_offset(&content, format!("A😀B{line_ending}").len()),
                Position::new(1, 0),
                "wrong position after {line_ending:?}"
            );
            assert_eq!(
                Position::from_offset(&content, format!("A😀B{line_ending}漢").len()),
                Position::new(1, 1),
                "wrong position after {line_ending:?}"
            );
        }
    }

    #[test]
    fn maps_offsets_inside_crlf_to_the_end_of_the_preceding_line() {
        let content = "A\r\nB";

        assert_eq!(
            Position::from_offset(content, "A\r".len()),
            Position::new(0, 1)
        );
        assert_eq!(
            Position::from_offset(content, "A\r\n".len()),
            Position::new(1, 0)
        );
    }
}
