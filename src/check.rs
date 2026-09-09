use crate::core::{diagnostic::Diagnostic, lines_inclusive::LinesInclusiveExt, position::Position};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    config::Config,
    document::Document,
    line_break::{BreakPoint, LineBreaker},
    spacing_checker::SpacingChecker,
};

pub(crate) fn check_one_file(
    config: &Config,
    document: &Document,
) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let mut diagnostics = Vec::new();

    // Initialize required components
    let breaker = LineBreaker::builder()
        .ambiguous_width(config.ambiguous_width)
        .max_width(config.max_width)
        .build()?;

    // Check line length problems
    for (line_index, line) in document.content.lines_inclusive().enumerate() {
        if let Some(diagnostic) = check_line_length(&breaker, document, line_index as u32, line) {
            diagnostics.push(diagnostic);
        }
    }

    // Check spacing problems
    let spacing_checker = SpacingChecker::new(config, document);
    diagnostics.extend(spacing_checker.check()?);

    Ok(diagnostics)
}

fn check_line_length(
    breaker: &LineBreaker,
    document: &Document,
    line_index: u32,
    line: &str,
) -> Option<Diagnostic> {
    let overflow_pos = match breaker.next_line_break(line) {
        BreakPoint::WrapPoint {
            overflow_pos,
            adjustment: _,
        } => overflow_pos,
        BreakPoint::EndOfLine(_) | BreakPoint::EndOfText(_) => {
            return None;
        }
    };
    let (precedings, followings) = line.split_at(overflow_pos);
    let column_index = precedings.encode_utf16().fold(0u32, |acc, _| acc + 1);
    let start = Position::new(line_index, column_index);
    let next_char_len = followings
        .graphemes(true)
        .next()
        .map(|s| s.encode_utf16().fold(0u32, |acc, _| acc + 1))
        .unwrap_or(0u32);
    let end = Position::new(line_index, column_index + next_char_len);
    Some(Diagnostic::new(
        document.filename.as_deref(),
        start,
        end,
        "W001".to_string(),
        format!("Line length exceeds {} characters", breaker.max_width()),
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::Grammar;

    use super::*;
    use crate::config::SpacingRule;

    #[test]
    fn check_one_file_checks_unparsed_markdown_documents() {
        let mut config = Config::default();
        config.spacing.alphabets = SpacingRule::Require;
        let document = Document::new("漢A", Grammar::Markdown, None::<String>);

        let diagnostics = check_one_file(&config, &document).expect("failed to check document");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W002");
    }

    #[test]
    fn check_one_file_reports_spacing_columns_from_line_start() {
        let mut config = Config::default();
        config.spacing.digits = SpacingRule::Require;

        let document = Document::new("# 漢1\n", Grammar::Markdown, Some("t.md"));

        let diagnostics = check_one_file(&config, &document).expect("failed to check document");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W002");
        assert_eq!(diagnostics[0].start, Position::new(0, 3));
        assert_eq!(diagnostics[0].end, Position::new(0, 4));
    }

    #[test]
    fn check_one_file_reports_spacing_columns_for_a_later_inline_node() {
        let mut config = Config::default();
        config.spacing.digits = SpacingRule::Require;

        let document = Document::new("# 見出し\n\n# 漢1\n", Grammar::Markdown, Some("t.md"));

        let diagnostics = check_one_file(&config, &document).expect("failed to check document");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W002");
        assert_eq!(diagnostics[0].start, Position::new(2, 3));
        assert_eq!(diagnostics[0].end, Position::new(2, 4));
    }

    #[test]
    fn check_one_file_reports_prohibited_spacing_without_panicking() {
        let mut config = Config::default();
        config.spacing.alphabets = SpacingRule::Prohibit;

        let document = Document::new("# 漢 A\n", Grammar::Markdown, Some("t.md"));

        let diagnostics = check_one_file(&config, &document).expect("failed to check document");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "W002");
        assert_eq!(diagnostics[0].start, Position::new(0, 3));
        assert_eq!(diagnostics[0].end, Position::new(0, 4));
    }

    #[test]
    fn check_one_file_reports_spacing_in_each_kind_of_visible_inline_prose() {
        let mut config = Config::default();
        config.spacing.alphabets = SpacingRule::Require;

        for (source, start) in [
            ("*漢A*", 2),
            ("~~A漢~~", 3),
            ("[漢A](destination)", 2),
            ("![漢A](image.png)", 3),
        ] {
            let document = Document::new(source, Grammar::Markdown, Some("t.md"));
            let diagnostics = check_one_file(&config, &document).expect("failed to check document");
            assert_eq!(
                diagnostics.len(),
                1,
                "unexpected diagnostics for {source:?}"
            );
            assert_eq!(diagnostics[0].start, Position::new(0, start));
            assert_eq!(diagnostics[0].end, Position::new(0, start + 1));
        }
    }

    #[test]
    fn check_one_file_reports_deletion_span_for_the_entire_ascii_space_run() {
        let mut config = Config::default();
        config.spacing.alphabets = SpacingRule::Prohibit;
        let document = Document::new("漢  A", Grammar::Markdown, Some("t.md"));

        let diagnostics = check_one_file(&config, &document).expect("failed to check document");
        assert_eq!(diagnostics[0].start, Position::new(0, 1));
        assert_eq!(diagnostics[0].end, Position::new(0, 3));
    }

    #[test]
    fn check_one_file_excludes_non_prose_and_unsafe_inline_constructs() {
        let mut config = Config::default();
        config.spacing.alphabets = SpacingRule::Require;
        let sources = [
            "`漢A`",
            "```\n漢A\n```",
            "[text](漢A \"漢A\")",
            "[text][漢A]",
            "<https://example.test/漢A>",
            "<foo@example.test>",
            "<a href=\"漢A\">",
            "&amp;漢",
            "\\*漢",
            "[漢A](broken",
            "`漢A",
        ];

        for source in sources {
            // Keep an eligible pair outside the excluded construct so this test
            // proves the checker is selecting prose, rather than finding no pair.
            let source_with_prose = format!("{source}\n\n漢A");
            let document = Document::new(&source_with_prose, Grammar::Markdown, Some("t.md"));
            let diagnostics = check_one_file(&config, &document)
                .expect("failed to check document")
                .into_iter()
                .filter(|diagnostic| diagnostic.code == "W002")
                .collect::<Vec<_>>();
            assert_eq!(
                diagnostics.len(),
                1,
                "unexpected diagnostics for {source:?}"
            );
            let prose_line = source.matches('\n').count() as u32 + 2;
            assert_eq!(diagnostics[0].start, Position::new(prose_line, 1));
            assert_eq!(diagnostics[0].end, Position::new(prose_line, 2));
        }
    }

    #[test]
    fn check_one_file_does_not_report_spacing_for_json_documents() {
        let mut config = Config::default();
        config.spacing.alphabets = SpacingRule::Require;
        let document = Document::new("{\"value\":\"漢A\"}", Grammar::Json, Some("t.json"));

        let diagnostics = check_one_file(&config, &document).expect("failed to check document");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "W002")
        );
    }
}
