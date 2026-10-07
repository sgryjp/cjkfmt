use super::position::Position;

/// Reusable physical-line and byte-offset facts for one immutable source snapshot.
#[derive(Debug)]
pub(crate) struct PhysicalSource<'a> {
    text: &'a str,
    lines: Vec<LineSpan>,
    utf16_columns: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
struct LineSpan {
    start: usize,
    content_end: usize,
    end: usize,
}

/// One physical line, with its content and existing terminator kept distinct.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PhysicalLine<'a> {
    text: &'a str,
    index: usize,
    span: LineSpan,
}

impl<'a> PhysicalSource<'a> {
    /// Indexes LF, CRLF, and bare-CR lines in `text` without creating a final
    /// iterator item after a terminating line ending.
    pub(crate) fn new(text: &'a str) -> Self {
        let bytes = text.as_bytes();
        let mut lines = Vec::new();
        let mut line_start = 0;
        let mut offset = 0;

        while offset < bytes.len() {
            let ending_length = match bytes[offset] {
                b'\r' if bytes.get(offset + 1) == Some(&b'\n') => 2,
                b'\r' | b'\n' => 1,
                _ => {
                    offset += 1;
                    continue;
                }
            };
            let end = offset + ending_length;
            lines.push(LineSpan {
                start: line_start,
                content_end: offset,
                end,
            });
            line_start = end;
            offset = end;
        }

        if line_start < text.len() {
            lines.push(LineSpan {
                start: line_start,
                content_end: text.len(),
                end: text.len(),
            });
        }

        // Direct byte-offset lookup avoids rescanning line prefixes for each
        // diagnostic endpoint; callers query only UTF-8 character boundaries.
        let mut utf16_columns = vec![0; text.len() + 1];
        for span in &lines {
            let mut column = 0u32;
            for (relative_offset, character) in text[span.start..span.content_end].char_indices() {
                let character_start = span.start + relative_offset;
                utf16_columns[character_start] = column;
                column = column.wrapping_add(character.len_utf16() as u32);
                utf16_columns[character_start + character.len_utf8()] = column;
            }
            if span.end - span.content_end == 2 {
                utf16_columns[span.content_end + 1] = column;
            }
        }

        Self {
            text,
            lines,
            utf16_columns,
        }
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    /// Iterates source lines including their existing terminators.
    pub(crate) fn lines(&self) -> impl Iterator<Item = PhysicalLine<'a>> + '_ {
        self.lines
            .iter()
            .copied()
            .enumerate()
            .map(move |(index, span)| PhysicalLine {
                text: self.text,
                index,
                span,
            })
    }

    /// Returns the byte offset's zero-based line and UTF-16 column.
    ///
    /// An offset between CR and LF maps to the preceding line's content end.
    pub(crate) fn position(&self, offset: usize) -> Position {
        assert!(offset <= self.text.len() && self.text.is_char_boundary(offset));

        let insertion = self.lines.partition_point(|line| line.start <= offset);
        let index = insertion.checked_sub(1).filter(|&index| {
            let span = self.lines[index];
            offset <= span.content_end
                || (span.end - span.content_end == 2 && offset == span.content_end + 1)
        });
        let line = index.unwrap_or(self.lines.len());
        let column_offset = index
            .filter(|&index| offset > self.lines[index].content_end)
            .map_or(offset, |index| self.lines[index].content_end);
        Position::new(line as u32, self.utf16_columns[column_offset])
    }

    /// Finds the physical line whose content contains a source offset.
    ///
    /// Content-end offsets are included. The offset between CR and LF is not;
    /// an offset after a final terminator belongs to the empty EOF line.
    pub(crate) fn line_containing_content_offset(&self, offset: usize) -> Option<PhysicalLine<'a>> {
        if offset > self.text.len() {
            return None;
        }

        let insertion = self.lines.partition_point(|line| line.start <= offset);
        if let Some(index) = insertion.checked_sub(1)
            && offset <= self.lines[index].content_end
        {
            return self.line_at(index);
        }

        (offset == self.text.len()).then(|| self.empty_eof_line())
    }

    fn line_at(&self, index: usize) -> Option<PhysicalLine<'a>> {
        self.lines.get(index).copied().map(|span| PhysicalLine {
            text: self.text,
            index,
            span,
        })
    }

    fn empty_eof_line(&self) -> PhysicalLine<'a> {
        let end = self.text.len();
        PhysicalLine {
            text: self.text,
            index: self.lines.len(),
            span: LineSpan {
                start: end,
                content_end: end,
                end,
            },
        }
    }
}

impl<'a> PhysicalLine<'a> {
    pub(crate) fn index(self) -> usize {
        self.index
    }

    pub(crate) fn start_offset(self) -> usize {
        self.span.start
    }

    pub(crate) fn content_end_offset(self) -> usize {
        self.span.content_end
    }

    pub(crate) fn content(self) -> &'a str {
        &self.text[self.span.start..self.span.content_end]
    }

    pub(crate) fn terminator(self) -> &'a str {
        &self.text[self.span.content_end..self.span.end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn indexes_content_and_terminator_spans_for_each_physical_line_ending() {
        let source = PhysicalSource::new("A\r\nB\rC\nD");
        let lines = source.lines().collect::<Vec<_>>();

        assert_eq!(
            lines
                .iter()
                .map(|line| (line.content(), line.terminator()))
                .collect::<Vec<_>>(),
            [("A", "\r\n"), ("B", "\r"), ("C", "\n"), ("D", "")]
        );
    }

    #[rstest]
    #[case("", vec![])]
    #[case("\rb", vec![("", "\r"), ("b", "")])]
    #[case("\nb", vec![("", "\n"), ("b", "")])]
    #[case("\r\nb", vec![("", "\r\n"), ("b", "")])]
    #[case("a\n", vec![("a", "\n")])]
    #[case("a\r", vec![("a", "\r")])]
    #[case("a\r\n", vec![("a", "\r\n")])]
    #[case("a\n\n", vec![("a", "\n"), ("", "\n")])]
    #[case("a", vec![("a", "")])]
    #[case("a\nb", vec![("a", "\n"), ("b", "")])]
    #[case("a\rb", vec![("a", "\r"), ("b", "")])]
    #[case("a\r\nb", vec![("a", "\r\n"), ("b", "")])]
    #[case("a\r亜", vec![("a", "\r"), ("亜", "")])]
    #[case("a\n亜", vec![("a", "\n"), ("亜", "")])]
    fn physical_line_iteration_preserves_terminator_inclusive_slices(
        #[case] text: &str,
        #[case] expected: Vec<(&str, &str)>,
    ) {
        let source = PhysicalSource::new(text);
        let actual = source
            .lines()
            .map(|line| (line.content(), line.terminator()))
            .collect::<Vec<_>>();

        assert_eq!(actual, expected, "wrong physical lines for {text:?}");
    }

    #[test]
    fn maps_utf16_positions_for_all_terminators_and_crlf_interiors() {
        for ending in ["\n", "\r\n", "\r"] {
            let text = format!("A😀B{ending}漢C");
            let source = PhysicalSource::new(&text);

            assert_eq!(source.position(0), Position::new(0, 0));
            assert_eq!(source.position("A😀".len()), Position::new(0, 3));
            if ending == "\r\n" {
                assert_eq!(source.position("A😀B\r".len()), Position::new(0, 4));
            }
            assert_eq!(
                source.position(format!("A😀B{ending}").len()),
                Position::new(1, 0)
            );
            assert_eq!(
                source.position(format!("A😀B{ending}漢").len()),
                Position::new(1, 1)
            );
        }
    }

    #[test]
    fn maps_crlf_interior_offsets_to_the_preceding_content_end() {
        let source = PhysicalSource::new("A\r\nB");

        assert_eq!(source.position("A\r".len()), Position::new(0, 1));
    }

    #[test]
    fn distinguishes_content_boundaries_from_terminator_interiors() {
        let source = PhysicalSource::new("a\r\nb");

        assert_eq!(
            source
                .line_containing_content_offset(1)
                .map(PhysicalLine::index),
            Some(0)
        );
        assert!(source.line_containing_content_offset(2).is_none());
        assert_eq!(
            source
                .line_containing_content_offset(3)
                .map(PhysicalLine::index),
            Some(1)
        );
    }

    #[test]
    fn treats_eof_after_a_final_terminator_as_an_empty_physical_line_for_queries() {
        let source = PhysicalSource::new("a\r\n");
        let eof_line = source
            .line_containing_content_offset(source.text().len())
            .unwrap();

        assert_eq!(eof_line.index(), 1);
        assert_eq!(eof_line.content(), "");
        assert_eq!(source.position(source.text().len()), Position::new(1, 0));
        assert_eq!(source.lines().count(), 1);
    }
}
