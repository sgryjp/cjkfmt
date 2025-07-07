/// An iterator over the lines of a string, as string slices.
///
/// The difference from `str::lines` is that line terminators are included
/// in the lines returned by the iterator.
pub(crate) struct LinesInclusive<'a> {
    index: usize,
    text: &'a str,
}

impl<'a> Iterator for LinesInclusive<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // Yield next line with end-of-line code
        for i in self.index..self.text.len() {
            match (self.text.get(i..i + 1), self.text.get(i + 1..i + 2)) {
                (Some("\r"), Some("\n")) => {
                    let (start, end) = (self.index, i + 2);
                    self.index = end;
                    return Some(&self.text[start..end]);
                }
                (Some("\r"), Some(_)) | (Some("\n"), Some(_)) => {
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
