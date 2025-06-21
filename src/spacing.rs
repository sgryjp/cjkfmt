use crate::_log::test_log;

#[derive(Debug, Clone, Copy, PartialEq)]
enum CharType {
    Cjk,
    NonCjk,
    Space,
}

pub fn search_possible_spacing_positions(text: &str) -> Vec<usize> {
    let mut indices = Vec::new();

    // Scan the text character by character to find possible position to insert a space.
    // This is a simple heuristic that checks if a CJK character is followed by a non-CJK character
    let mut char_iterator = text.char_indices();
    let Some(mut prev_type) = char_iterator.next().map(|(_, c)| char_type(c)) else {
        return indices;
    };
    for (index, curr_char) in char_iterator {
        // Check if this is a candidate position to insert a space
        let curr_type = char_type(curr_char);
        if prev_type == CharType::Cjk && curr_type == CharType::NonCjk
            || prev_type == CharType::NonCjk && curr_type == CharType::Cjk
        {
            indices.push(index);
        }
        test_log!(
            "{:?}[{:2}] --> {:?} ({:?}, {:?})",
            text,
            index,
            curr_char,
            prev_type,
            curr_type
        );

        // Update the previous character type
        prev_type = curr_type;
    }

    indices
}

fn char_type(c: char) -> CharType {
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
        | '\u{AC00}'..='\u{D7AF}'
         => CharType::Cjk,

        // Whitespace characters
        ' ' | '\r' | '\n' => CharType::Space,

        // Other characters
        _ => CharType::NonCjk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[test]
    fn test_char_type() {
        assert!(char_type('中') == CharType::Cjk);
        assert!(char_type('漢') == CharType::Cjk);
        assert!(char_type('a') == CharType::NonCjk);
        assert!(char_type('1') == CharType::NonCjk);
        assert!(char_type(' ') == CharType::Space);
    }

    #[rstest]
    #[case("漢漢", vec![])]
    #[case("漢a", vec![3])]
    #[case("漢 a", vec![])]
    #[case("a漢", vec![1])]
    #[case("a 漢", vec![])]
    #[case("漢\n", vec![])]
    #[case("a\n", vec![])]
    fn test_check_spacing_in_a_line(#[case] text: &str, #[case] indices: Vec<usize>) {
        assert_eq!(search_possible_spacing_positions(text), indices);
    }
}
