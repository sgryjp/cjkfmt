/// An iterator over the lines of a string, as string slices.
///
/// The difference from `str::lines` is that line terminators are included
/// in the lines returned by the iterator.
pub struct LinesInclusive<'a> {
    index: usize,
    text: &'a str,
}

impl<'a> Iterator for LinesInclusive<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // Yield next line with end-of-line code
        for i in self.index..self.text.len() {
            match (self.text.get(i..i + 1), self.text.get(i + 1..i + 2)) {
                // CR+LF
                (Some("\r"), Some("\n")) => {
                    let (start, end) = (self.index, i + 2);
                    self.index = end;
                    return Some(&self.text[start..end]);
                }
                // CR followed by a single byte character
                (Some("\r"), Some(_))
                // CR followed by a multi-byte character or end of text
                | (Some("\r"), None)
                // LF followed by a single byte character
                | (Some("\n"), Some(_))
                // LF followed by a multi-byte character or end of text
                | (Some("\n"), None) => {
                    let (start, end) = (self.index, i + 1);
                    self.index = end;
                    return Some(&self.text[start..end]);
                }
                _ => {}
            }
        }

        // Yield the trailing part following the last line ending
        if self.index < self.text.len() {
            let (start, end) = (self.index, self.text.len());
            self.index = self.text.len();
            return Some(&self.text[start..end]);
        }

        None
    }
}

impl<'a> LinesInclusive<'a> {
    #[inline]
    pub fn new(text: &'a str) -> Self {
        LinesInclusive { index: 0, text }
    }
}

/// Extension trait to add `lines_inclusive` to `&str`
///
/// [`lines_inclusive`]: core::lines_inclusive::LinesInclusive
pub trait LinesInclusiveExt {
    /// Returns an iterator over the lines of a string including the line endings.
    fn lines_inclusive(&self) -> LinesInclusive<'_>;
}

impl LinesInclusiveExt for str {
    fn lines_inclusive(&self) -> LinesInclusive<'_> {
        LinesInclusive::new(self)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("", vec![])]
    #[case("\rb", vec!["\r", "b"])]
    #[case("\nb", vec!["\n", "b"])]
    #[case("\r\nb", vec!["\r\n", "b"])]
    fn test_empty_line(#[case] input: &str, #[case] expected: Vec<&str>) {
        assert_eq!(input.lines_inclusive().collect::<Vec<&str>>(), expected);
    }

    #[rstest]
    #[case("a\n", vec!["a\n"])]
    #[case("a\r", vec!["a\r"])]
    #[case("a\r\n", vec!["a\r\n"])]
    fn test_lines_with_final_line_ending(#[case] input: &str, #[case] expected: Vec<&str>) {
        assert_eq!(input.lines_inclusive().collect::<Vec<&str>>(), expected);
    }

    #[rstest]
    #[case("a", vec!["a"])]
    #[case("a\nb", vec!["a\n", "b"])]
    #[case("a\rb", vec!["a\r", "b"])]
    #[case("a\r\nb", vec!["a\r\n", "b"])]
    fn test_lines_without_final_line_ending(#[case] input: &str, #[case] expected: Vec<&str>) {
        assert_eq!(input.lines_inclusive().collect::<Vec<&str>>(), expected);
    }

    #[rstest]
    #[case("a\r亜", vec!["a\r", "亜"])]
    #[case("a\n亜", vec!["a\n", "亜"])]
    fn test_line_endings_followed_by_a_multibyte_char(
        #[case] input: &str,
        #[case] expected: Vec<&str>,
    ) {
        assert_eq!(input.lines_inclusive().collect::<Vec<&str>>(), expected);
    }
}
