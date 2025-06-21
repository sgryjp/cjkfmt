//! Define [`LineBreaker`] that finds a line break with adherence to kinsoku rule.
use unicode_linebreak::{BreakClass, break_property};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::_log::test_log;

/// Grapheme clusters prohibited at the start of a line.
pub const PROHIBITED_START: &str = ")]｝〕〉》」』】〙〗〟'\"｠»\
    ヽヾーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻\
    ‐゠–〜\
    ？ ! ‼ ⁇ ⁈ ⁉\
    ・、:;,\
    。.";

/// Grapheme clusters prohibited at the end of a line.
pub const PROHIBITED_END: &str = "([｛〔〈《「『【〘〖〝'\"｟«";

/// A line break point detected by [`LineBreaker`].
#[derive(Debug, PartialEq, Eq)]
pub enum BreakPoint {
    /// A line break caused by an EOL code such as CR+LF or LF.
    EndOfLine(usize),

    /// A line break detected by the [`LineBreaker`].
    ///
    /// This is a point where the line exceeds the maximum width and should be
    /// wrapped. `WrapPoint` contains the following fields:
    ///
    /// - `overflow_pos`: The position of the character that caused the overflow.
    ///   This is the byte index of the character in the original line.
    /// - `adjustment`: The number of bytes to backtrack to find an acceptable break point.
    WrapPoint {
        overflow_pos: usize,
        adjustment: usize,
    },

    /// End of the input text.
    EndOfText(usize),
}

/// Find a line break with adherence to kinsoku rule.
///
/// Use [`LineBreaker::builder`] to create a new instance of [`LineBreaker`].
#[derive(Debug)]
pub struct LineBreaker {
    max_width: u32,
    prohibited_start: Vec<String>,
    prohibited_end: Vec<String>,
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
    pub fn max_width(mut self, max_width: u32) -> Self {
        self.line_breaker.max_width = max_width;
        self
    }

    /// Sets the grapheme clusters that are prohibited at the start of a line.
    ///
    /// This method replaces the default set of prohibited grapheme clusters.
    fn prohibited_start<S: AsRef<str>>(mut self, graphemes: S) -> Self {
        self.line_breaker.prohibited_start = graphemes
            .as_ref()
            .graphemes(true)
            .map(|s| s.to_owned())
            .collect();
        self
    }

    /// Sets the grapheme clusters that are prohibited at the end of a line.
    ///
    /// This method replaces the default set of prohibited grapheme clusters.
    fn prohibited_end<S: AsRef<str>>(mut self, graphemes: S) -> Self {
        self.line_breaker.prohibited_end = graphemes
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
                "max_width out of range: {} (cannot be below 2)",
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
                prohibited_start: Vec::new(),
                prohibited_end: Vec::new(),
            },
        }
        .prohibited_start(PROHIBITED_START)
        .prohibited_end(PROHIBITED_END)
    }

    /// Returns the maximum width of a line.
    pub fn max_width(&self) -> u32 {
        self.max_width
    }

    // TODO: Remove this method
    fn prohibited_start<'a>(&'a self) -> Vec<&'a str> {
        self.prohibited_start
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&'a str>>()
    }

    fn prohibited_end<'a>(&'a self) -> Vec<&'a str> {
        self.prohibited_end
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&'a str>>()
    }

    /// Finds a line break in the given line and returns its byte index.
    pub fn next_line_break(&self, line: &str) -> BreakPoint {
        test_log!("next_line_break() {:?}", line);

        let mut graphemes: Vec<&str> = Vec::with_capacity(128);
        let mut acc_width = 0u32;
        for (i, grapheme) in line.grapheme_indices(true) {
            // Stop if reached EOL.
            if grapheme == "\r" || grapheme == "\n" {
                test_log!("  {i:02} {:?} eol", grapheme);
                return BreakPoint::EndOfLine(i + 1);
            } else if grapheme == "\r\n" {
                test_log!("  {i:02} {:?} eol", grapheme);
                return BreakPoint::EndOfLine(i + 2);
            }

            // Test whether rendering this grapheme cluster will exceed the limit or not
            let width = grapheme.width_cjk() as u32;
            if self.max_width < acc_width + width {
                test_log!("  {i:02} {:?} !!", grapheme);
                if let Some(nbytes_seek_back) =
                    self.num_bytes_to_seek_back(graphemes.as_slice(), grapheme)
                {
                    test_log!(
                        "  split at {:02} --> {:?}",
                        i - nbytes_seek_back,
                        line.split_at_checked(i - nbytes_seek_back)
                            .expect("invalid split point")
                    );
                    return BreakPoint::WrapPoint {
                        overflow_pos: i,
                        adjustment: nbytes_seek_back,
                    };
                }
            }

            // Go to next grapheme cluster
            test_log!("  {i:02} {:?}", grapheme);
            graphemes.push(grapheme);
            acc_width += width;
        }
        BreakPoint::EndOfText(line.len())
    }

    fn num_bytes_to_seek_back(
        &self,
        preceding_graphemes: &[&str],
        following_grapheme: &str,
    ) -> Option<usize> {
        debug_assert!(!preceding_graphemes.is_empty());

        let mut nbytes_to_rewind = 0;
        let mut following = following_grapheme;
        for grapheme in preceding_graphemes.iter().rev() {
            // Seek back if not breakable according to UAX#14
            if !is_breakable(grapheme, following) {
                let grapheme_len = grapheme.len();
                nbytes_to_rewind += grapheme_len;
                test_log!("    rewind {grapheme_len}: {grapheme:?} {following:?} (not breakable)",);
                following = grapheme;
                continue;
            }

            // Seek back if the preceding character is prohibited at line end
            if self.prohibited_end().contains(grapheme) {
                let grapheme_len = grapheme.len();
                nbytes_to_rewind += grapheme_len;
                test_log!("    rewind {grapheme_len}: {grapheme:?} {following:?} (prohibited_end)");
                following = grapheme;
                continue;
            }

            // Seek back if the following character is prohibited at line start
            if self.prohibited_start().contains(&following) {
                let grapheme_len = grapheme.len();
                nbytes_to_rewind += grapheme_len;
                test_log!(
                    "    rewind {grapheme_len}: {grapheme:?} {following:?} (prohibited_start)"
                );
                following = grapheme;
                continue;
            }

            test_log!("    break: {grapheme:?} {following:?}");
            return Some(nbytes_to_rewind);
        }
        test_log!("  cannot rewind anymore");
        None
    }
}

/// Check whether a line break is allowed between the given grapheme clusters.
///
/// This function is based on UAX#14 so kinsoku rules are not considered.
#[allow(clippy::let_and_return)]
fn is_breakable(preceding: &str, following: &str) -> bool {
    debug_assert!(!preceding.is_empty());
    debug_assert!(!following.is_empty());

    let preceding_break_property = preceding
        .chars()
        .last()
        .map(|c| break_property(c as u32))
        .expect("`preceding` must be non-empty");
    let following_break_property = following
        .chars()
        .next()
        .map(|c| break_property(c as u32))
        .expect("`following` must be non-empty");
    let breakable = match (preceding_break_property, following_break_property) {
        (BreakClass::After, _) => false,
        (_, BreakClass::Before) => false,
        (BreakClass::BeforeAndAfter, _) => false,
        (_, BreakClass::BeforeAndAfter) => false,
        (BreakClass::Alphabetic, BreakClass::Alphabetic) => false,
        (_, BreakClass::Space) => false,
        (_, _) => true,
    };
    // log!(
    //     "    {preceding:?}({:?}) {following:?}({:?}) --> {breakable}",
    //     preceding_break_property,
    //     following_break_property
    // );
    breakable
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest]
    // not breakable between alphabetic characters
    #[case("a", "a", false)]
    #[case("a", "あ", true)]
    // not breakable before a space
    #[case("a", " ", false)]
    #[case(" ", "a", true)]
    fn is_breakable_normal(
        #[case] preceding: &str,
        #[case] following: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(is_breakable(preceding, following), expected);
    }

    #[test]
    fn max_width() {
        assert!(LineBreaker::builder().max_width(0).build().is_err());
        assert!(LineBreaker::builder().max_width(1).build().is_err());
        assert!(LineBreaker::builder().max_width(2).build().is_ok());
        assert!(LineBreaker::builder().max_width(3).build().is_ok());
    }

    #[rstest]
    #[case(10, "あ「い」う", BreakPoint::EndOfText(15))]
    #[case(9, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 12, adjustment: 0 })]
    #[case(8, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 12, adjustment: 0 })]
    #[case(7, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 9, adjustment: 6 })]
    #[case(6, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 9, adjustment: 6 })]
    #[case(5, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 6, adjustment: 3 })]
    #[case(4, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 6, adjustment: 3 })]
    #[case(3, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 3, adjustment: 0 })]
    #[case(2, "あ「い」う", BreakPoint::WrapPoint { overflow_pos: 3, adjustment: 0 })]
    fn next_line_break(
        #[case] max_width: u32,
        #[case] line: &str,
        #[case] expected: BreakPoint,
    ) -> anyhow::Result<()> {
        let line_breaker = LineBreaker::builder().max_width(max_width).build()?;
        let actual = line_breaker.next_line_break(line);
        assert_eq!(expected, actual);
        Ok(())
    }

    #[rstest]
    #[case(2, "foo\rbar", BreakPoint::EndOfLine(4))]
    #[case(3, "foo\rbar", BreakPoint::EndOfLine(4))]
    #[case(4, "foo\rbar", BreakPoint::EndOfLine(4))]
    #[case(5, "foo\rbar", BreakPoint::EndOfLine(4))]
    #[case(5, "foo\nbar", BreakPoint::EndOfLine(4))]
    #[case(5, "foo\r\nbar", BreakPoint::EndOfLine(5))]
    fn next_line_break_eol(
        #[case] max_width: u32,
        #[case] line: &str,
        #[case] expected: BreakPoint,
    ) -> anyhow::Result<()> {
        let line_breaker = LineBreaker::builder().max_width(max_width).build()?;
        let actual = line_breaker.next_line_break(line);
        assert_eq!(expected, actual);
        Ok(())
    }

    // Cat + ZWJ + Black Large Square
    #[rstest]
    #[case(8, "あ「🐈‍⬛」う", BreakPoint::WrapPoint { overflow_pos: 19, adjustment: 0 })]
    #[case(7, "あ「🐈‍⬛」う", BreakPoint::WrapPoint { overflow_pos: 16, adjustment: 13 })]
    fn next_line_break_composite(
        #[case] max_width: u32,
        #[case] line: &str,
        #[case] expected: BreakPoint,
    ) -> anyhow::Result<()> {
        let line_breaker = LineBreaker::builder().max_width(max_width).build()?;
        let actual = line_breaker.next_line_break(line);
        assert_eq!(expected, actual);
        Ok(())
    }

    #[rstest]
    #[case(11, "あfoo barい", BreakPoint::EndOfText(13))]
    #[case(10, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 10, adjustment: 0 })]
    #[case(9, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 10, adjustment: 0 })]
    #[case(8, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 9, adjustment: 2 })]
    #[case(7, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 8, adjustment: 1 })]
    #[case(6, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 7, adjustment: 0 })]
    #[case(5, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 6, adjustment: 3 })]
    #[case(4, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 5, adjustment: 2 })]
    #[case(3, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 4, adjustment: 1 })]
    #[case(2, "あfoo barい", BreakPoint::WrapPoint { overflow_pos: 3, adjustment: 0 })]
    fn next_line_break_western_word_wrap(
        #[case] max_width: u32,
        #[case] line: &str,
        #[case] expected: BreakPoint,
    ) -> anyhow::Result<()> {
        let line_breaker = LineBreaker::builder().max_width(max_width).build()?;
        let actual = line_breaker.next_line_break(line);
        assert_eq!(expected, actual);
        Ok(())
    }

    #[rstest]
    #[case(8, "あ「い」", "う", Some(0))]
    #[case(6, "あ「い", "」", Some(6))]
    #[case(3, "あ「", "い", Some(3))]
    #[case(2, "あ", "「", Some(0))]
    #[case(2, "」", "「", Some(0))]
    #[case(2, "「", "「", None)]
    fn num_bytes_to_seek_back(
        #[case] max_width: u32,
        #[case] preceding_graphemes: &str,
        #[case] following_grapheme: &str,
        #[case] expected: Option<usize>,
    ) -> anyhow::Result<()> {
        let preceding_graphemes: Vec<&str> = preceding_graphemes.graphemes(true).collect();
        let line_breaker = LineBreaker::builder().max_width(max_width).build()?;

        let actual = line_breaker.num_bytes_to_seek_back(&preceding_graphemes, following_grapheme);
        assert_eq!(actual, expected);

        Ok(())
    }
}
