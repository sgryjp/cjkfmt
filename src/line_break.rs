//! Define [`LineBreaker`] that finds a line break with adherence to kinsoku rule.
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A set of grapheme clusters that are prohibited at the start of a line.
pub const PROHIBITED_LINE_START_GRAPHEMES: &str = ")]｝〕〉》」』】〙〗〟'\"｠»\
     ヽヾーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻\
     ‐゠–〜\
     ？ ! ‼ ⁇ ⁈ ⁉\
     ・、:;,\
     。.";

/// A set of grapheme clusters that are prohibited at the end of a line.
pub const PROHIBITED_LINE_END_GRAPHEMES: &str = "([｛〔〈《「『【〘〖〝'\"｟«";

/// Find a line break with adherence to kinsoku rule.
///
/// Use [`LineBreaker::builder`] to create a new instance of [`LineBreaker`].
#[derive(Debug)]
pub struct LineBreaker {
    max_width: usize,
    graphemes_prohibited_at_line_start: Vec<String>, // TODO: Use &str
    graphemes_prohibited_at_line_end: Vec<String>,
}

/// Build a [`LineBreaker`].
pub struct LineBreakerBuilder {
    line_breaker: LineBreaker,
}

impl LineBreakerBuilder {
    /// Sets the maximum width of a line.
    ///
    /// The width is measured in terms of fullwidth characters.
    /// For example, the width of "あ" is 2, and the width of "a" is 1.
    pub fn max_width(mut self, max_width: usize) -> Self {
        self.line_breaker.max_width = max_width;
        self
    }

    /// Sets the grapheme clusters that are prohibited at the start of a line.
    ///
    /// This method replaces the default set of prohibited grapheme clusters.
    fn graphemes_prohibited_at_line_start<S: AsRef<str>>(mut self, graphemes: S) -> Self {
        self.line_breaker.graphemes_prohibited_at_line_start = graphemes
            .as_ref()
            .graphemes(true)
            .map(|s| s.to_owned())
            .collect();
        self
    }

    /// Sets the grapheme clusters that are prohibited at the end of a line.
    ///
    /// This method replaces the default set of prohibited grapheme clusters.
    fn graphemes_prohibited_at_line_end<S: AsRef<str>>(mut self, graphemes: S) -> Self {
        self.line_breaker.graphemes_prohibited_at_line_end = graphemes
            .as_ref()
            .graphemes(true)
            .map(|s| s.to_string())
            .collect();
        self
    }

    /// Finish building and returns a [`LineBreaker`].
    pub fn build(self) -> anyhow::Result<LineBreaker> {
        if self.line_breaker.max_width < 2 {
            anyhow::bail!(
                "max_width out of range: {}. Cannot be below 2.",
                self.line_breaker.max_width
            )
        } else {
            Ok(self.line_breaker)
        }
    }
}

impl LineBreaker {
    /// Builds a new [`LineBreaker`] with default settings.
    pub fn builder() -> LineBreakerBuilder {
        LineBreakerBuilder {
            line_breaker: LineBreaker {
                max_width: 80,
                graphemes_prohibited_at_line_start: Vec::new(),
                graphemes_prohibited_at_line_end: Vec::new(),
            },
        }
        .graphemes_prohibited_at_line_start(PROHIBITED_LINE_START_GRAPHEMES)
        .graphemes_prohibited_at_line_end(PROHIBITED_LINE_END_GRAPHEMES)
    }

    // TODO: Remove this method
    fn graphemes_prohibited_at_line_start<'a>(&'a self) -> Vec<&'a str> {
        self.graphemes_prohibited_at_line_start
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&'a str>>()
    }

    fn graphemes_prohibited_at_line_end<'a>(&'a self) -> Vec<&'a str> {
        self.graphemes_prohibited_at_line_end
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&'a str>>()
    }

    /// Finds a line break in the given line and returns its index.
    pub fn next_line_break(&self, line: &str) -> Option<usize> {
        let mut graphemes: Vec<&str> = Vec::with_capacity(128);
        let mut acc_width: usize = 0;
        for (i, grapheme) in line.grapheme_indices(true) {
            // Stop if reached EOL.
            if grapheme.ends_with('\n') {
                return None;
            }

            // Test whether rendering this grapheme cluster will exceed the limit or not
            let width = grapheme.width_cjk();
            if self.max_width < acc_width + width {
                let nbytes_seek_back = self.num_bytes_to_seek_back(graphemes.as_slice(), grapheme);
                return Some(i - nbytes_seek_back);
            }

            // Go to next grapheme cluster
            graphemes.push(grapheme);
            acc_width += width;
        }
        None
    }

    fn num_bytes_to_seek_back(
        &self,
        preceding_graphemes: &[&str],
        following_grapheme: &str,
    ) -> usize {
        debug_assert!(!preceding_graphemes.is_empty());

        let mut nbytes_to_rewind = 0;
        let mut following = following_grapheme;
        for grapheme in preceding_graphemes.iter().skip(1).rev() {
            if self.graphemes_prohibited_at_line_end().contains(grapheme) {
                nbytes_to_rewind += grapheme.len();
                following = grapheme;
                continue;
            }

            if self
                .graphemes_prohibited_at_line_start()
                .contains(&following)
            {
                nbytes_to_rewind += grapheme.len();
                following = grapheme;
                continue;
            }

            break;
        }
        nbytes_to_rewind
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_max_width() {
        assert!(LineBreaker::builder().max_width(0).build().is_err());
        assert!(LineBreaker::builder().max_width(1).build().is_err());
        assert!(LineBreaker::builder().max_width(2).build().is_ok());
        assert!(LineBreaker::builder().max_width(3).build().is_ok());
    }

    #[rstest]
    #[case(10, "あ「い」う", None)]
    #[case(9, "あ「い」う", Some(12))]
    #[case(8, "あ「い」う", Some(12))]
    #[case(7, "あ「い」う", Some(3))]
    #[case(6, "あ「い」う", Some(3))]
    #[case(5, "あ「い」う", Some(3))]
    #[case(4, "あ「い」う", Some(3))]
    #[case(3, "あ「い」う", Some(3))]
    #[case(2, "あ「い」う", Some(3))]
    fn test_next_line_break(
        #[case] max_width: usize,
        #[case] line: &str,
        #[case] expected: Option<usize>,
    ) -> anyhow::Result<()> {
        let line_breaker = LineBreaker::builder().max_width(max_width).build()?;
        let actual = line_breaker.next_line_break(line);
        assert_eq!(expected, actual);
        Ok(())
    }

    #[rstest]
    #[case(8, "あ「い」", "う", 0)]
    #[case(6, "あ「い", "」", 6)]
    #[case(3, "あ「", "い", 3)]
    #[case(2, "あ", "「", 0)]
    #[case(2, "」", "「", 0)]
    #[case(2, "「", "「", 0)]
    fn test_num_bytes_to_seek_back(
        #[case] max_width: usize,
        #[case] preceding_graphemes: &str,
        #[case] following_grapheme: &str,
        #[case] expected: usize,
    ) -> anyhow::Result<()> {
        let preceding_graphemes: Vec<&str> = preceding_graphemes.graphemes(true).collect();
        let line_breaker = LineBreaker::builder().max_width(max_width).build()?;

        let actual = line_breaker.num_bytes_to_seek_back(&preceding_graphemes, following_grapheme);
        assert_eq!(actual, expected);

        Ok(())
    }
}
