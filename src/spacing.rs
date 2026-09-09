use std::{ops::Range, str::CharIndices};

use unicode_general_category::{GeneralCategory, get_general_category};

use crate::config::{Config, SpacingRule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextEdit {
    pub(crate) range: Range<usize>,
    pub(crate) replacement: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CharType {
    Cjk,
    Latin,
    Digit,
    Space,
    Other,
}

/// Returns the spacing edits needed for a piece of text.
///
/// The ranges are UTF-8 byte ranges relative to `text`. Markdown syntax is
/// intentionally not considered here; callers that understand a syntax tree
/// filter these edits before applying them.
pub(crate) fn spacing_edits(config: &Config, text: &str) -> Vec<TextEdit> {
    let characters = text_characters(text);
    let mut edits = Vec::new();

    for pair in characters.windows(2) {
        let [previous, current] = pair else {
            unreachable!();
        };

        match (previous.kind, current.kind) {
            pair if is_spacing_pair(pair.0, pair.1)
                && spacing_rule(config, pair.0, pair.1) == SpacingRule::Require =>
            {
                edits.push(TextEdit {
                    range: current.start..current.start,
                    replacement: " ".to_string(),
                });
            }
            _ => {}
        }
    }

    // A run of ASCII spaces is handled as one edit. In particular, do not
    // treat tabs or line endings as part of a deletable run.
    let mut index = 0;
    while index < characters.len() {
        if characters[index].character != ' ' {
            index += 1;
            continue;
        }

        let start = index;
        while index < characters.len() && characters[index].character == ' ' {
            index += 1;
        }
        let end = index;
        if start == 0 || end == characters.len() {
            continue;
        }

        let left = characters[start - 1];
        let right = characters[end];
        if is_spacing_pair(left.kind, right.kind)
            && spacing_rule(config, left.kind, right.kind) == SpacingRule::Prohibit
        {
            edits.push(TextEdit {
                range: left.end..right.start,
                replacement: String::new(),
            });
        }
    }

    edits
}

#[derive(Debug, Clone, Copy)]
struct TextCharacter {
    start: usize,
    end: usize,
    character: char,
    kind: CharType,
}

fn text_characters(text: &str) -> Vec<TextCharacter> {
    let mut characters = Vec::new();
    let mut indices: CharIndices<'_> = text.char_indices();
    while let Some((start, character)) = indices.next() {
        let end = indices
            .clone()
            .next()
            .map_or(text.len(), |(index, _)| index);
        characters.push(TextCharacter {
            start,
            end,
            character,
            kind: char_type(character),
        });
    }
    characters
}

fn is_spacing_pair(left: CharType, right: CharType) -> bool {
    matches!(
        (left, right),
        (CharType::Cjk, CharType::Digit)
            | (CharType::Digit, CharType::Cjk)
            | (CharType::Cjk, CharType::Latin)
            | (CharType::Latin, CharType::Cjk)
    )
}

fn spacing_rule(config: &Config, left: CharType, right: CharType) -> SpacingRule {
    match (left, right) {
        (CharType::Cjk, CharType::Digit) | (CharType::Digit, CharType::Cjk) => {
            config.spacing.digits
        }
        (CharType::Cjk, CharType::Latin) | (CharType::Latin, CharType::Cjk) => {
            config.spacing.alphabets
        }
        _ => SpacingRule::Ignore,
    }
}

fn char_type(c: char) -> CharType {
    // Only ASCII spaces are editable. Other whitespace must also prevent a
    // spacing pair from spanning it, including U+3000 in the broad CJK range.
    match c {
        ' ' | '\r' | '\n' => return CharType::Space,
        _ if c.is_whitespace() => return CharType::Other,
        _ => {}
    }

    // TODO: Refine the character set by reviewing https://www.unicode.org/charts/
    match c {
        // CJK Unified Ideographs
        '\u{4E00}'..='\u{9FFF}'
        // CJK Unified Ideographs Extension A
        | '\u{3400}'..='\u{4DBF}'
        // CJK Unified Ideographs Extension B
        | '\u{20000}'..='\u{2A6DF}'
        // CJK Unified Ideographs Extension C
        | '\u{2A700}'..='\u{2B73F}'
        // CJK Unified Ideographs Extension D
        | '\u{2B740}'..='\u{2B81F}'
        // CJK Unified Ideographs Extension E
        | '\u{2B820}'..='\u{2CEAF}'
        // CJK Unified Ideographs Extension F
        | '\u{2CEB0}'..='\u{2EBEF}'
        // CJK Unified Ideographs Extension G
        | '\u{30000}'..='\u{3134F}'
        // CJK Unified Ideographs Extension H
        | '\u{31350}'..='\u{323AF}'
        // CJK Unified Ideographs Extension I
        | '\u{2EBF0}'..='\u{2EE5D}'
        // CJK Radicals Supplement
        | '\u{2E80}'..='\u{2EFF}'
        // CJK Symbols and Punctuation
        | '\u{3000}'..='\u{303F}'
        // Hiragana: U+3040–U+309F
        | '\u{3040}'..='\u{309F}'
        // Katakana: U+30A0–U+30FF
        | '\u{30A0}'..='\u{30FF}'
        // Bopomofo: U+3100–U+312F
        | '\u{3100}'..='\u{312F}'
        // Hangul Syllables: U+AC00–U+D7AF
        | '\u{AC00}'..='\u{D7AF}' => match get_general_category(c) {
            // Exclude punctuation characters.
            GeneralCategory::ClosePunctuation
            | GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::OtherPunctuation => CharType::Other,
            _ => CharType::Cjk,
        },

        // Basic Latin : Uppercase and lowercase letters
        'A'..='Z'
        | 'a'..='z'
        // Latin-1 Supplement
        | '\u{00C0}'..='\u{00FF}'
        // Latin Extended-A
        | '\u{0100}'..='\u{017F}'
        // Latin Extended-B
        | '\u{0180}'..='\u{024F}'
        // Latin Extended Additional
        | '\u{1E00}'..='\u{1EFF}'
        // IPA Extensions
        | '\u{0250}'..='\u{02AF}'
        // Spacing Modifier Letters
        | '\u{02B0}'..='\u{02FF}'
        // Combining Diacritical Marks
        | '\u{0300}'..='\u{036F}'
        // Combining Diacritical Marks Extended
        | '\u{1AB0}'..='\u{1AFF}'
        // Combining Diacritical Marks Supplement
        | '\u{1DC0}'..='\u{1DFF}'
        // Latin Extended-C
        | '\u{2C60}'..='\u{2C7F}'
        // Latin Extended-D
        | '\u{A720}'..='\u{A7FF}'
        // Latin Extended-E
        | '\u{AB30}'..='\u{AB6F}'
        // Latin Extended-F
        | '\u{10780}'..='\u{107BF}'
        // Latin Extended-G
        | '\u{1DF00}'..='\u{1DFFF}' => CharType::Latin,

        // Half-width digits
        '0'..='9' => CharType::Digit,

        _ => CharType::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(alphabets: SpacingRule, digits: SpacingRule) -> Config {
        let mut config = Config::default();
        config.spacing.alphabets = alphabets;
        config.spacing.digits = digits;
        config
    }

    fn edits(config: &Config, text: &str) -> Vec<(Range<usize>, String)> {
        spacing_edits(config, text)
            .into_iter()
            .map(|edit| (edit.range, edit.replacement))
            .collect()
    }

    #[test]
    fn require_inserts_at_utf8_byte_boundaries_in_both_directions() {
        let config = make_config(SpacingRule::Require, SpacingRule::Require);
        assert_eq!(
            edits(&config, "漢A1漢"),
            vec![(3..3, " ".to_string()), (5..5, " ".to_string())]
        );
    }

    #[test]
    fn require_does_not_duplicate_existing_ascii_space() {
        let config = make_config(SpacingRule::Require, SpacingRule::Require);
        assert!(spacing_edits(&config, "漢 A").is_empty());
    }

    #[test]
    fn prohibit_deletes_a_complete_ascii_space_run() {
        let config = make_config(SpacingRule::Prohibit, SpacingRule::Prohibit);
        assert_eq!(edits(&config, "漢   A  1"), vec![(3..6, String::new())]);
    }

    #[test]
    fn prohibit_does_not_cross_non_ascii_space_or_line_endings() {
        let config = make_config(SpacingRule::Prohibit, SpacingRule::Prohibit);
        for text in ["漢\tA", "漢\nA", "漢\r\nA", "漢\u{00a0}A", "漢\u{3000}A"] {
            assert!(spacing_edits(&config, text).is_empty(), "changed {text:?}");
        }
    }

    #[test]
    fn alphabet_and_digit_rules_are_independent() {
        let config = make_config(SpacingRule::Require, SpacingRule::Ignore);
        assert_eq!(
            edits(&config, "漢A漢1"),
            vec![(3..3, " ".to_string()), (4..4, " ".to_string())]
        );

        let digit_config = make_config(SpacingRule::Ignore, SpacingRule::Prohibit);
        assert_eq!(
            edits(&digit_config, "漢 A 漢  1"),
            vec![(9..11, String::new())]
        );
    }

    #[test]
    fn ignore_leaves_both_kinds_unchanged() {
        let config = make_config(SpacingRule::Ignore, SpacingRule::Ignore);
        assert!(spacing_edits(&config, "漢A 漢 1").is_empty());
    }

    #[test]
    fn character_types_keep_punctuation_out_of_spacing_pairs() {
        assert_eq!(char_type('中'), CharType::Cjk);
        assert_eq!(char_type('漢'), CharType::Cjk);
        assert_eq!(char_type('a'), CharType::Latin);
        assert_eq!(char_type('1'), CharType::Digit);
        assert_eq!(char_type(' '), CharType::Space);
        assert_eq!(char_type('。'), CharType::Other);
        assert_eq!(char_type('\u{3000}'), CharType::Other);
    }
}
