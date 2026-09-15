//! Internal plans and validation shared by language formatting policies.
//!
//! Policies describe source-relative changes.  This module keeps the
//! orchestration layer from having to know whether a change came from a
//! parser, while still rejecting plans that cannot be applied atomically.

// The policy seam and break-plan helpers are introduced before their concrete
// adapters and planner consumers in the migration sequence. The item-level
// allowances below are limited to those staged, not-yet-reachable pieces.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::config::SpacingConfig;

/// The spacing configuration visible to a language formatting policy.
///
/// This alias deliberately keeps policy code independent of the rest of the
/// formatter configuration.  Width and line-breaking settings are not part
/// of a policy's input.
#[allow(dead_code)]
pub(crate) type SpacingRules = SpacingConfig;

/// A source-relative replacement planned by a language policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
    pub(crate) range: Range<usize>,
    pub(crate) replacement: String,
}

/// A syntax-approved place where the common line-break planner may split a
/// physical source line.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BreakOpportunity {
    /// Document-relative range in the post-spacing source.
    pub(crate) replace: Range<usize>,

    /// Minimal syntax scaffolding for the generated continuation line.
    /// This must not contain a line-ending sequence.
    pub(crate) continuation: String,
}

/// Errors raised when a policy violates the internal formatting contract.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum LanguageFormatError {
    #[error("{kind} range {range:?} is outside the source")]
    InvalidRange {
        kind: &'static str,
        range: Range<usize>,
    },

    #[error("{kind} range {range:?} is not on UTF-8 and grapheme boundaries")]
    InvalidBoundary {
        kind: &'static str,
        range: Range<usize>,
    },

    #[allow(dead_code)]
    #[error("{kind} range {range:?} crosses an existing physical line")]
    CrossesPhysicalLine {
        kind: &'static str,
        range: Range<usize>,
    },

    #[allow(dead_code)]
    #[error("break opportunity range {range:?} contains non-horizontal whitespace")]
    InvalidBreakReplacement { range: Range<usize> },

    #[allow(dead_code)]
    #[error("break opportunity continuation contains a line-ending sequence")]
    ContinuationContainsLineEnding,

    #[error("overlapping or duplicate {kind} ranges: {previous:?} and {current:?}")]
    OverlappingRanges {
        kind: &'static str,
        previous: Range<usize>,
        current: Range<usize>,
    },

    #[allow(dead_code)]
    #[error("formatter policy failed: {0}")]
    Policy(String),
}

/// The internal seam implemented by each supported language adapter.
#[allow(dead_code)]
pub(crate) trait LanguageFormatPolicy {
    fn plan_spacing_edits(
        &self,
        source: &str,
        rules: &SpacingRules,
    ) -> Result<Vec<TextEdit>, LanguageFormatError>;

    fn plan_break_opportunities(
        &self,
        source: &str,
    ) -> Result<Vec<BreakOpportunity>, LanguageFormatError>;
}

/// Validate and canonicalize source-relative spacing edits.
pub(crate) fn validate_text_edits(
    source: &str,
    edits: &mut [TextEdit],
) -> Result<(), LanguageFormatError> {
    for edit in edits.iter() {
        validate_range(source, "text edit", &edit.range)?;
    }
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    validate_order("text edit", edits.iter().map(|edit| &edit.range))
}

/// Validate and canonicalize document-relative break opportunities.
#[allow(dead_code)]
pub(crate) fn validate_break_opportunities(
    source: &str,
    opportunities: &mut [BreakOpportunity],
) -> Result<(), LanguageFormatError> {
    for opportunity in opportunities.iter() {
        let range = &opportunity.replace;
        // Check physical source content before grapheme validation so a
        // character boundary inside CRLF (for example, byte 2 in `a\r\nb`)
        // cannot be mistaken for a valid insertion point.
        if range.start <= range.end
            && range.end <= source.len()
            && source.is_char_boundary(range.start)
            && source.is_char_boundary(range.end)
            && !is_one_physical_line(source, range)
        {
            return Err(LanguageFormatError::CrossesPhysicalLine {
                kind: "break opportunity",
                range: range.clone(),
            });
        }
        validate_range(source, "break opportunity", range)?;
        if !opportunity.replace.is_empty()
            && !source[opportunity.replace.clone()]
                .chars()
                .all(is_horizontal_whitespace)
        {
            return Err(LanguageFormatError::InvalidBreakReplacement {
                range: opportunity.replace.clone(),
            });
        }
        if contains_line_ending(&opportunity.continuation) {
            return Err(LanguageFormatError::ContinuationContainsLineEnding);
        }
    }

    opportunities.sort_by(|left, right| {
        (left.replace.start, left.replace.end, &left.continuation).cmp(&(
            right.replace.start,
            right.replace.end,
            &right.continuation,
        ))
    });
    validate_order(
        "break opportunity",
        opportunities.iter().map(|opportunity| &opportunity.replace),
    )
}

/// Apply validated source-relative text edits in one output construction.
pub(crate) fn apply_text_edits(
    source: &str,
    edits: &[TextEdit],
) -> Result<String, LanguageFormatError> {
    let mut canonical = edits.to_vec();
    validate_text_edits(source, &mut canonical)?;

    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for edit in canonical {
        output.push_str(&source[cursor..edit.range.start]);
        output.push_str(&edit.replacement);
        cursor = edit.range.end;
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}

/// Apply selected break opportunities without exposing a partially built
/// document.  The caller is responsible for selecting opportunities; this
/// utility validates their source-relative structure before constructing the
/// result.
#[allow(dead_code)]
pub(crate) fn apply_break_opportunities(
    source: &str,
    opportunities: &[BreakOpportunity],
    line_ending: &str,
) -> Result<String, LanguageFormatError> {
    if !matches!(line_ending, "\n" | "\r" | "\r\n") {
        return Err(LanguageFormatError::Policy(
            "invalid inserted line ending".to_string(),
        ));
    }

    let mut canonical = opportunities.to_vec();
    validate_break_opportunities(source, &mut canonical)?;

    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for opportunity in canonical {
        output.push_str(&source[cursor..opportunity.replace.start]);
        output.push_str(line_ending);
        output.push_str(&opportunity.continuation);
        cursor = opportunity.replace.end;
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}

fn validate_range(
    source: &str,
    kind: &'static str,
    range: &Range<usize>,
) -> Result<(), LanguageFormatError> {
    if range.start > range.end || range.end > source.len() {
        return Err(LanguageFormatError::InvalidRange {
            kind,
            range: range.clone(),
        });
    }
    if !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
        || !is_grapheme_boundary(source, range.start)
        || !is_grapheme_boundary(source, range.end)
    {
        return Err(LanguageFormatError::InvalidBoundary {
            kind,
            range: range.clone(),
        });
    }
    Ok(())
}

fn is_grapheme_boundary(source: &str, offset: usize) -> bool {
    offset == 0
        || offset == source.len()
        || source
            .grapheme_indices(true)
            .any(|(start, _)| start == offset)
}

fn validate_order<'a>(
    kind: &'static str,
    ranges: impl Iterator<Item = &'a Range<usize>>,
) -> Result<(), LanguageFormatError> {
    let ranges = ranges.collect::<Vec<_>>();
    for pair in ranges.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        if previous.end > current.start
            || (previous.is_empty() && current.is_empty() && previous.start == current.start)
            || (previous.start == current.start && (!previous.is_empty() || !current.is_empty()))
        {
            return Err(LanguageFormatError::OverlappingRanges {
                kind,
                previous: previous.clone(),
                current: current.clone(),
            });
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn is_one_physical_line(source: &str, range: &Range<usize>) -> bool {
    // A replace range must be contained in content, not in or across an
    // existing CR, LF, or CRLF separator.  Empty ranges at a content boundary
    // are valid and are what policies use for adjacent-token seams.
    let Some(start_line) = physical_line_containing(source, range.start) else {
        return false;
    };
    let Some(end_line) = physical_line_containing(source, range.end) else {
        return false;
    };
    start_line == end_line
}

#[allow(dead_code)]
fn physical_line_containing(source: &str, offset: usize) -> Option<usize> {
    let mut line = 0;
    let mut cursor = 0;
    while cursor <= source.len() {
        let remaining = &source[cursor..];
        let next_ending = remaining
            .find(['\r', '\n'])
            .map(|relative| cursor + relative);
        let content_end = next_ending.unwrap_or(source.len());
        if offset <= content_end {
            return Some(line);
        }
        let ending_start = content_end;
        if ending_start == source.len() {
            return None;
        }
        let ending_length = if source[ending_start..].starts_with("\r\n") {
            2
        } else {
            1
        };
        if offset < ending_start + ending_length {
            return None;
        }
        cursor = ending_start + ending_length;
        line += 1;
    }
    None
}

#[allow(dead_code)]
fn is_horizontal_whitespace(character: char) -> bool {
    character.is_whitespace() && !is_line_ending_character(character)
}

#[allow(dead_code)]
fn contains_line_ending(text: &str) -> bool {
    text.chars().any(is_line_ending_character)
}

#[allow(dead_code)]
fn is_line_ending_character(character: char) -> bool {
    matches!(
        character,
        '\r' | '\n' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(start: usize, end: usize) -> TextEdit {
        TextEdit {
            range: start..end,
            replacement: String::new(),
        }
    }

    fn opportunity(start: usize, end: usize) -> BreakOpportunity {
        BreakOpportunity {
            replace: start..end,
            continuation: String::new(),
        }
    }

    #[test]
    fn text_edits_are_sorted_and_applied_atomically() {
        let edits = [
            TextEdit {
                range: 3..3,
                replacement: " ".to_string(),
            },
            TextEdit {
                range: 0..1,
                replacement: "X".to_string(),
            },
        ];
        assert_eq!(apply_text_edits("a bc", &edits).unwrap(), "X b c");
    }

    #[test]
    fn invalid_text_edit_fails_before_any_output_is_constructed() {
        let edits = [edit(0, 99), edit(0, 0)];
        let error = apply_text_edits("source", &edits).unwrap_err();
        assert!(matches!(error, LanguageFormatError::InvalidRange { .. }));
    }

    #[test]
    fn text_edits_require_utf8_and_grapheme_boundaries() {
        let source = "e\u{301}";
        let edits = [edit(1, 1)];
        assert!(matches!(
            validate_text_edits(source, &mut edits.clone()),
            Err(LanguageFormatError::InvalidBoundary { .. })
        ));
    }

    #[test]
    fn break_opportunities_accept_empty_and_horizontal_whitespace_ranges() {
        let mut opportunities = vec![opportunity(0, 0), opportunity(1, 2)];
        validate_break_opportunities("a b", &mut opportunities).unwrap();
    }

    #[test]
    fn empty_break_opportunities_cannot_be_inside_a_physical_terminator() {
        let mut inside_crlf = vec![opportunity(2, 2)];
        assert_eq!(
            validate_break_opportunities("a\r\nb", &mut inside_crlf),
            Err(LanguageFormatError::CrossesPhysicalLine {
                kind: "break opportunity",
                range: 2..2,
            })
        );
    }

    #[test]
    fn break_opportunities_reject_non_whitespace_and_line_crossing_ranges() {
        let mut non_whitespace = vec![opportunity(0, 1)];
        assert!(matches!(
            validate_break_opportunities("abc", &mut non_whitespace),
            Err(LanguageFormatError::InvalidBreakReplacement { .. })
        ));

        let mut crossing = vec![opportunity(1, 3)];
        assert!(matches!(
            validate_break_opportunities("a\nb", &mut crossing),
            Err(LanguageFormatError::CrossesPhysicalLine { .. })
        ));
    }

    #[test]
    fn break_opportunities_reject_duplicates_and_overlap() {
        let mut duplicates = vec![opportunity(1, 1), opportunity(1, 1)];
        assert!(matches!(
            validate_break_opportunities("abc", &mut duplicates),
            Err(LanguageFormatError::OverlappingRanges { .. })
        ));

        let mut overlap = vec![opportunity(1, 3), opportunity(2, 2)];
        assert!(matches!(
            validate_break_opportunities("a  bc", &mut overlap),
            Err(LanguageFormatError::OverlappingRanges { .. })
        ));
    }

    #[test]
    fn continuation_rejects_every_supported_line_ending_character() {
        for ending in [
            "\r", "\n", "\u{000B}", "\u{000C}", "\u{0085}", "\u{2028}", "\u{2029}",
        ] {
            let mut opportunities = vec![BreakOpportunity {
                replace: 1..1,
                continuation: format!("prefix{ending}"),
            }];
            assert!(matches!(
                validate_break_opportunities("ab", &mut opportunities),
                Err(LanguageFormatError::ContinuationContainsLineEnding),
            ));
        }
    }

    #[test]
    fn applying_break_opportunities_preserves_source_relative_coordinates() {
        let opportunities = [BreakOpportunity {
            replace: 1..2,
            continuation: "> ".to_string(),
        }];
        assert_eq!(
            apply_break_opportunities("a bc", &opportunities, "\r\n").unwrap(),
            "a\r\n> bc"
        );
    }
}
