//! Language-independent planning for physical line breaks.
use std::{ops::Range, sync::Arc};

use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_linebreak::{BreakClass, break_property};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{_log::test_log, config::AmbiguousWidth};

/// Grapheme clusters prohibited at the start of a line.
pub const PROHIBITED_START: &str = ")]｝〕〉》」』】〙〗〟'\"｠»\
    ヽヾーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻\
    ‐゠–〜\
    ？ ! ‼ ⁇ ⁈ ⁉\
    ・、:;,\
    。.";

/// Grapheme clusters prohibited at the end of a line.
pub const PROHIBITED_END: &str = "([｛〔〈《「『【〘〖〝'\"｟«";

/// A language-approved, line-relative place where a physical line may break.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineRelativeBreakOpportunity {
    pub(crate) replace: Range<usize>,
    pub(crate) continuation: String,
}

/// A break selected by [`LineBreakPlanner`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectedBreak {
    pub(crate) replace: Range<usize>,
    pub(crate) line_ending: String,
    pub(crate) continuation: String,
}

/// Plans physical line breaks independently of any source language.
#[derive(Debug)]
pub(crate) struct LineBreakPlanner {
    ambiguous_width: AmbiguousWidth,
    max_width: u32,
    prohibited_start: Arc<[String]>,
    prohibited_end: Arc<[String]>,
}

/// Builds a [`LineBreakPlanner`].
pub(crate) struct LineBreakPlannerBuilder {
    planner: LineBreakPlanner,
}

impl LineBreakPlannerBuilder {
    /// Sets how to treat width of characters in the Ambiguous category.
    pub(crate) fn ambiguous_width(mut self, ambiguous_width: AmbiguousWidth) -> Self {
        self.planner.ambiguous_width = ambiguous_width;
        self
    }

    /// Sets the maximum width of a line.
    pub(crate) fn max_width(mut self, max_width: u32) -> Self {
        self.planner.max_width = max_width;
        self
    }

    fn prohibited_start<S: AsRef<str>>(mut self, graphemes: S) -> Self {
        self.planner.prohibited_start = graphemes
            .as_ref()
            .graphemes(true)
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .into();
        self
    }

    fn prohibited_end<S: AsRef<str>>(mut self, graphemes: S) -> Self {
        self.planner.prohibited_end = graphemes
            .as_ref()
            .graphemes(true)
            .map(str::to_owned)
            .collect::<Vec<_>>()
            .into();
        self
    }

    /// Finishes building the planner.
    pub(crate) fn build(self) -> anyhow::Result<LineBreakPlanner> {
        if self.planner.max_width < 2 {
            anyhow::bail!(
                "max_width out of range: {} (cannot be below 2)",
                self.planner.max_width
            )
        }
        Ok(self.planner)
    }
}

impl LineBreakPlanner {
    /// Builds a planner with the default settings.
    pub(crate) fn builder() -> LineBreakPlannerBuilder {
        LineBreakPlannerBuilder {
            planner: LineBreakPlanner {
                ambiguous_width: AmbiguousWidth::Wide,
                max_width: 80,
                prohibited_start: Vec::new().into(),
                prohibited_end: Vec::new().into(),
            },
        }
        .prohibited_start(PROHIBITED_START)
        .prohibited_end(PROHIBITED_END)
    }

    pub(crate) fn max_width(&self) -> u32 {
        self.max_width
    }

    /// Returns the first grapheme whose addition exceeds the configured width.
    /// Existing physical line endings are not measured as source content.
    pub(crate) fn first_overflow(&self, line: &str) -> Option<usize> {
        test_log!("first_overflow() {:?}", line);
        let content_end = content_end(line);
        let mut width = 0;
        for (offset, grapheme) in line[..content_end].grapheme_indices(true) {
            width += self.grapheme_width(grapheme);
            if width > self.max_width {
                return Some(offset);
            }
        }
        None
    }

    /// Selects safe breaks without mutating `line`.
    ///
    /// The input opportunities are already constrained by the source-language
    /// policy. This planner adds the common UAX #14 and kinsoku constraints,
    /// then chooses the rightmost safe point before overflow or the first safe
    /// point after indivisible overlong content.
    pub(crate) fn plan_breaks(
        &self,
        line: &str,
        opportunities: &[LineRelativeBreakOpportunity],
        inserted_line_ending: &str,
    ) -> Vec<SelectedBreak> {
        let end = content_end(line);
        let mut cursor = 0;
        let mut candidate_index = 0;
        let mut continuation = String::new();
        let mut selected = Vec::new();

        while cursor < end {
            let mut scan_cursor = cursor;
            let mut width = self.text_width(&continuation);
            let mut overflowed = width > self.max_width;
            let mut latest_safe = None;
            let mut selected_index = None;

            while scan_cursor < end {
                while let Some(opportunity) = opportunities.get(candidate_index) {
                    if opportunity.replace.start < scan_cursor
                        || opportunity.replace.start <= cursor
                        || opportunity.replace.start > opportunity.replace.end
                        || opportunity.replace.end > end
                    {
                        candidate_index += 1;
                    } else {
                        break;
                    }
                }

                while let Some(opportunity) = opportunities.get(candidate_index) {
                    if opportunity.replace.start != scan_cursor {
                        break;
                    }
                    let current_index = candidate_index;
                    candidate_index += 1;
                    if !self.is_safe_opportunity(line, opportunity, end) {
                        continue;
                    }
                    if overflowed {
                        selected_index = Some(current_index);
                        break;
                    }
                    if width <= self.max_width {
                        latest_safe = Some(current_index);
                    }
                }
                if selected_index.is_some() {
                    break;
                }

                let Some((_, grapheme)) = line[scan_cursor..end].grapheme_indices(true).next()
                else {
                    break;
                };
                width += self.grapheme_width(grapheme);
                scan_cursor += grapheme.len();
                if width > self.max_width {
                    selected_index = latest_safe.take();
                    if selected_index.is_some() {
                        break;
                    }
                    overflowed = true;
                }
            }

            let Some(selected_index) = selected_index else {
                break;
            };
            let opportunity = &opportunities[selected_index];
            if opportunity.replace.end <= cursor {
                break;
            }
            cursor = opportunity.replace.end;
            continuation = opportunity.continuation.clone();
            selected.push(SelectedBreak {
                replace: opportunity.replace.clone(),
                line_ending: inserted_line_ending.to_owned(),
                continuation: continuation.clone(),
            });
            // Candidates scanned after the selected point belong to the
            // generated continuation line and must be reconsidered there.
            candidate_index = selected_index + 1;
        }
        selected
    }

    fn is_safe_opportunity(
        &self,
        line: &str,
        opportunity: &LineRelativeBreakOpportunity,
        end: usize,
    ) -> bool {
        let range = &opportunity.replace;
        if range.start == 0 || range.end >= end || range.end > line.len() {
            return false;
        }
        let Some(preceding) = line[..range.start].graphemes(true).next_back() else {
            return false;
        };
        let Some(following) = line[range.end..end].graphemes(true).next() else {
            return false;
        };
        if self.prohibited_end.iter().any(|item| item == preceding)
            || self.prohibited_start.iter().any(|item| item == following)
        {
            return false;
        }

        if is_no_break_grapheme(preceding)
            || is_no_break_grapheme(following)
            || is_immediately_adjacent_to_zero_width_joiner(&line[..range.start], true)
            || is_immediately_adjacent_to_zero_width_joiner(&line[range.end..end], false)
            || is_no_break_adjacent_to_modifier(&line[..range.start], true)
            || is_no_break_adjacent_to_modifier(&line[range.end..end], false)
        {
            return false;
        }
        if range.is_empty() {
            return is_breakable(preceding, following);
        }

        let replaced = &line[range.clone()];
        if !replaced.chars().all(is_horizontal_whitespace) {
            return false;
        }
        let Some(first_replaced) = replaced.graphemes(true).next() else {
            return false;
        };
        let Some(last_replaced) = replaced.graphemes(true).next_back() else {
            return false;
        };
        if is_no_break_grapheme(first_replaced) || is_no_break_grapheme(last_replaced) {
            return false;
        }
        is_breakable(preceding, first_replaced) || is_breakable(last_replaced, following)
    }

    fn text_width(&self, text: &str) -> u32 {
        text.graphemes(true)
            .map(|grapheme| self.grapheme_width(grapheme))
            .sum()
    }

    fn grapheme_width(&self, grapheme: &str) -> u32 {
        (match self.ambiguous_width {
            AmbiguousWidth::Narrow => grapheme.width(),
            AmbiguousWidth::Wide => grapheme.width_cjk(),
        }) as u32
    }
}

fn content_end(line: &str) -> usize {
    line.find(['\r', '\n']).unwrap_or(line.len())
}

/// Check whether a line break is allowed between the given grapheme clusters.
/// This function is based on UAX #14 so kinsoku rules are not considered.
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
    match (preceding_break_property, following_break_property) {
        (BreakClass::After, _) => false,
        (_, BreakClass::Before) => false,
        (BreakClass::BeforeAndAfter, _) => false,
        (_, BreakClass::BeforeAndAfter) => false,
        (BreakClass::NonBreakingGlue | BreakClass::WordJoiner | BreakClass::ZeroWidthJoiner, _) => {
            false
        }
        (_, BreakClass::NonBreakingGlue | BreakClass::WordJoiner | BreakClass::ZeroWidthJoiner) => {
            false
        }
        (BreakClass::Alphabetic, BreakClass::Alphabetic) => false,
        (_, BreakClass::Space) => false,
        (_, _) => true,
    }
}

fn is_no_break_grapheme(grapheme: &str) -> bool {
    grapheme.chars().any(is_no_break_character)
}

fn is_immediately_adjacent_to_zero_width_joiner(text: &str, reverse: bool) -> bool {
    if reverse {
        text.ends_with('\u{200d}')
    } else {
        text.starts_with('\u{200d}')
    }
}

fn is_no_break_adjacent_to_modifier(text: &str, reverse: bool) -> bool {
    if reverse {
        modifier_run_has_no_break(text.chars().rev())
    } else {
        modifier_run_has_no_break(text.chars())
    }
}

fn modifier_run_has_no_break(characters: impl Iterator<Item = char>) -> bool {
    let mut saw_modifier = false;
    for character in characters {
        if is_combining_mark_or_variation_selector(character) {
            saw_modifier = true;
        } else {
            return saw_modifier && (is_no_break_character(character) || character == '\u{200d}');
        }
    }
    false
}

fn is_no_break_character(character: char) -> bool {
    matches!(character, '\u{00a0}' | '\u{202f}')
        || matches!(
            break_property(character as u32),
            BreakClass::NonBreakingGlue | BreakClass::WordJoiner
        )
}

fn is_combining_mark_or_variation_selector(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
    ) || matches!(character, '\u{fe00}'..='\u{fe0f}' | '\u{e0100}'..='\u{e01ef}')
}

fn is_horizontal_whitespace(character: char) -> bool {
    character.is_whitespace() && !matches!(character, '\r' | '\n')
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn planner(max_width: u32) -> LineBreakPlanner {
        LineBreakPlanner::builder()
            .max_width(max_width)
            .build()
            .unwrap()
    }

    fn opportunity(start: usize, end: usize, continuation: &str) -> LineRelativeBreakOpportunity {
        LineRelativeBreakOpportunity {
            replace: start..end,
            continuation: continuation.to_owned(),
        }
    }

    fn grapheme_seams(line: &str) -> Vec<LineRelativeBreakOpportunity> {
        line.grapheme_indices(true)
            .map(|(offset, _)| offset)
            .filter(|&offset| offset > 0)
            .map(|offset| opportunity(offset, offset, ""))
            .collect()
    }

    #[rstest]
    #[case("a", "a", false)]
    #[case("a", "あ", true)]
    #[case("a", " ", false)]
    #[case(" ", "a", true)]
    fn uax14_breakability(
        #[case] preceding: &str,
        #[case] following: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(is_breakable(preceding, following), expected);
    }

    #[test]
    fn first_overflow_reports_physical_overflow_without_needing_a_safe_break() {
        assert_eq!(planner(2).first_overflow("abcde あ\n"), Some(2));
    }

    #[test]
    fn first_overflow_honors_ambiguous_width() {
        for (ambiguous_width, expected) in [
            (AmbiguousWidth::Narrow, None),
            (AmbiguousWidth::Wide, Some(1)),
        ] {
            let planner = LineBreakPlanner::builder()
                .ambiguous_width(ambiguous_width)
                .max_width(2)
                .build()
                .unwrap();
            assert_eq!(planner.first_overflow("1※"), expected);
        }
    }

    #[test]
    fn plan_breaks_selects_the_rightmost_safe_opportunity_before_overflow() {
        let planner = planner(4);
        let opportunities = [opportunity(3, 3, ""), opportunity(6, 6, "")];

        let selected = planner.plan_breaks("ああああ", &opportunities, "\n");

        assert_eq!(selected[0].replace, 6..6);
    }

    #[test]
    fn plan_breaks_counts_continuation_prefix_width_on_generated_lines() {
        let planner = planner(4);
        let opportunities = [
            opportunity(3, 3, "> "),
            opportunity(6, 6, "> "),
            opportunity(9, 9, "> "),
        ];

        let selected = planner.plan_breaks("ああああ", &opportunities, "\n");

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].replace, 6..6);
        assert_eq!(selected[1].replace, 9..9);
        assert_eq!(selected[1].continuation, "> ");
    }

    #[test]
    fn plan_breaks_resumes_at_the_first_safe_opportunity_after_overlong_content() {
        let planner = planner(2);
        let opportunities = [opportunity(7, 7, "")];

        let selected = planner.plan_breaks("abcdef ああ", &opportunities, "\n");

        assert_eq!(selected[0].replace, 7..7);
        assert_eq!(planner.first_overflow("abcdef ああ"), Some(2));
    }

    #[test]
    fn plan_breaks_preserves_graphemes_uax14_and_kinsoku() {
        let planner = planner(7);
        let line = "あ「🐈‍⬛」う";
        let opportunities = grapheme_seams(line);

        let selected = planner.plan_breaks(line, &opportunities, "\n");

        assert_eq!(selected[0].replace, 3..3);
        assert_eq!(&line[..selected[0].replace.start], "あ");
    }

    #[rstest]
    #[case("a\u{00a0}b", 1..3)]
    #[case("a\u{202f}b", 1..4)]
    #[case("a\u{2060}b", 1..4)]
    #[case("a\u{200d}b", 1..4)]
    #[case("a\u{00a0} b", 3..4)]
    #[case("a\u{202f} b", 4..5)]
    #[case("a\u{2060} b", 4..5)]
    #[case("a\u{200d} b", 4..5)]
    fn plan_breaks_does_not_split_at_glue_or_joiners(
        #[case] line: &str,
        #[case] replace: Range<usize>,
    ) {
        let selected =
            planner(2).plan_breaks(line, &[opportunity(replace.start, replace.end, "")], "\n");

        assert!(selected.is_empty());
    }

    #[rstest]
    #[case("漢\u{00a0}\u{0308}漢")]
    #[case("漢\u{2060}\u{0308}漢")]
    #[case("漢\u{200d}\u{0308}漢")]
    #[case("漢\u{00a0}\u{fe0f}漢")]
    #[case("漢\u{2060}\u{fe0f}漢")]
    #[case("漢\u{200d}\u{fe0f}漢")]
    fn plan_breaks_does_not_split_empty_seams_adjacent_to_modified_glue_or_joiners(
        #[case] line: &str,
    ) {
        let opportunities = grapheme_seams(line);

        assert!(
            planner(2)
                .plan_breaks(line, &opportunities, "\n")
                .is_empty()
        );
    }

    #[test]
    fn plan_breaks_allows_a_seam_after_a_completed_emoji_zwj_cluster() {
        let line = "👩‍👩 bbbb";
        let space = line.find(' ').unwrap();
        let selected = planner(4).plan_breaks(line, &[opportunity(space, space + 1, "")], "\n");

        assert_eq!(
            selected,
            [SelectedBreak {
                replace: space..space + 1,
                line_ending: "\n".to_owned(),
                continuation: String::new(),
            }]
        );
    }

    #[rstest]
    #[case("👩‍ bbbb")]
    #[case("👩 \u{200d}bbbb")]
    fn plan_breaks_rejects_seams_immediately_adjacent_to_a_zero_width_joiner(#[case] line: &str) {
        let space = line.find(' ').unwrap();

        assert!(
            planner(2)
                .plan_breaks(line, &[opportunity(space, space + 1, "")], "\n")
                .is_empty()
        );
    }

    #[test]
    fn max_width_must_allow_at_least_one_fullwidth_character() {
        assert!(LineBreakPlanner::builder().max_width(1).build().is_err());
        assert!(LineBreakPlanner::builder().max_width(2).build().is_ok());
    }

    #[test]
    fn line_endings_are_not_considered_breakable_content() {
        assert_eq!(planner(2).first_overflow("あ\r\n"), None);
    }
}
