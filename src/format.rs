use crate::core::lines_inclusive::LinesInclusiveExt;
use crate::{
    config::Config,
    formatting::{
        BreakOpportunity, LanguageFormatError, LanguageFormatPolicy, apply_text_edits,
        markdown::MarkdownFormatPolicy, validate_break_opportunities,
    },
    language::Language,
    line_break::{LineBreakPlanner, LineRelativeBreakOpportunity},
};

/// An error produced while constructing or running a [`Formatter`].
#[derive(Debug, thiserror::Error)]
pub(crate) enum FormatError {
    #[error("invalid formatter configuration: {0}")]
    Configuration(#[source] anyhow::Error),

    #[error("language formatting plan is invalid: {0}")]
    Language(#[source] LanguageFormatError),
}

/// Formats one complete document without exposing an intermediate output.
#[derive(Debug)]
pub(crate) struct Formatter {
    config: Config,
    line_breaker: LineBreakPlanner,
}

impl Formatter {
    /// Creates a formatter after validating the configuration it will use.
    pub(crate) fn new(config: &Config) -> Result<Self, FormatError> {
        let line_breaker = LineBreakPlanner::builder()
            .ambiguous_width(config.ambiguous_width)
            .max_width(config.max_width)
            .build()
            .map_err(FormatError::Configuration)?;
        Ok(Self {
            config: config.clone(),
            line_breaker,
        })
    }

    /// Formats a complete document, or returns it unchanged when no language
    /// was selected. The result is constructed before it is returned.
    pub(crate) fn format(
        &self,
        language: Option<Language>,
        source: &str,
    ) -> Result<String, FormatError> {
        let Some(language) = language else {
            return Ok(source.to_owned());
        };

        self.format_known_language(language, source)
    }

    fn format_known_language(
        &self,
        language: Language,
        source: &str,
    ) -> Result<String, FormatError> {
        // Markdown is planned against S0, reparsed against S1, and only then
        // passed to the common line-break planner. JSON retains its legacy
        // wrapping path until the JSON policy migration step.
        if language == Language::Markdown {
            let policy = MarkdownFormatPolicy;
            let edits = policy
                .plan_spacing_edits(source, &self.config.spacing)
                .map_err(FormatError::Language)?;
            let content = apply_text_edits(source, &edits).map_err(FormatError::Language)?;
            let mut opportunities = policy
                .plan_break_opportunities(&content)
                .map_err(FormatError::Language)?;
            validate_break_opportunities(&content, &mut opportunities)
                .map_err(FormatError::Language)?;
            return self.format_with_opportunities(&content, &opportunities);
        }

        let content = source.to_owned();
        self.format_lines(&content, |_, line| {
            self.line_breaker.legacy_opportunities(line)
        })
    }

    fn format_with_opportunities(
        &self,
        content: &str,
        opportunities: &[BreakOpportunity],
    ) -> Result<String, FormatError> {
        self.format_lines(content, |line_start, line| {
            let content_end = line.find(['\r', '\n']).unwrap_or(line.len());
            let line_end = line_start + content_end;
            opportunities
                .iter()
                .filter(|opportunity| {
                    opportunity.replace.start >= line_start && opportunity.replace.end <= line_end
                })
                .map(|opportunity| LineRelativeBreakOpportunity {
                    replace: (opportunity.replace.start - line_start)
                        ..(opportunity.replace.end - line_start),
                    continuation: opportunity.continuation.clone(),
                })
                .collect::<Vec<_>>()
        })
    }

    fn format_lines<F>(
        &self,
        content: &str,
        mut opportunities_for_line: F,
    ) -> Result<String, FormatError>
    where
        F: FnMut(usize, &str) -> Vec<LineRelativeBreakOpportunity>,
    {
        let mut formatted = String::with_capacity(content.len());
        let mut source_offset = 0;
        // An unterminated final line inherits the preceding physical line's
        // terminator. A single-line document has no preceding terminator, so
        // it uses LF as the default.
        let mut previous_line_ending = "\n";
        // Construct the whole result in memory so formatting failures cannot
        // expose a partially transformed document to the caller.
        for line in content.lines_inclusive() {
            let (line_ending, is_terminated) = if line.ends_with("\r\n") {
                ("\r\n", true)
            } else if line.ends_with('\r') {
                ("\r", true)
            } else if line.ends_with('\n') {
                ("\n", true)
            } else {
                (previous_line_ending, false)
            };
            if is_terminated {
                previous_line_ending = line_ending;
            }
            let opportunities = opportunities_for_line(source_offset, line);
            let breaks = self
                .line_breaker
                .plan_breaks(line, &opportunities, line_ending);
            let mut cursor = 0;
            for selected in breaks {
                formatted.push_str(&line[cursor..selected.replace.start]);
                formatted.push_str(&selected.line_ending);
                formatted.push_str(&selected.continuation);
                cursor = selected.replace.end;
            }
            formatted.push_str(&line[cursor..]);
            source_offset += line.len();
        }
        Ok(formatted)
    }
}

pub(crate) fn format_one_file<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    language: Option<Language>,
    content: &str,
) -> Result<(), anyhow::Error> {
    let formatter = Formatter::new(config)?;
    let formatted = formatter.format(language, content)?;
    stdout.write_all(formatted.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SpacingRule;

    fn config() -> Config {
        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        config
    }

    fn format(language: Option<crate::language::Language>, source: &str) -> String {
        Formatter::new(&config())
            .unwrap()
            .format(language, source)
            .unwrap()
    }

    fn has_node_kind(node: tree_sitter::Node<'_>, kind: &str) -> bool {
        if node.kind() == kind {
            return true;
        }
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .any(|child| has_node_kind(child, kind))
    }

    #[test]
    fn formatter_leaves_source_byte_identical_when_no_language_is_selected() {
        let mut config = config();
        config.max_width = 2;
        let source = "漢A one two three\r\n";
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(formatter.format(None, source).unwrap(), source);
    }

    #[test]
    fn format_one_file_writes_nothing_when_formatter_creation_fails() {
        let mut config = config();
        config.max_width = 1;
        let mut output = Vec::new();

        assert!(format_one_file(&mut output, &config, Some(Language::Json), "source").is_err());
        assert!(output.is_empty());
    }

    #[test]
    fn format_applies_configured_spacing_to_markdown_prose() {
        assert_eq!(
            format(Some(crate::language::Language::Markdown), "漢A\n"),
            "漢 A\n"
        );
    }

    #[test]
    fn format_preserves_spacing_in_non_markdown_input() {
        let source = "{\"value\":\"漢A\"}\n";
        assert_eq!(
            format(Some(crate::language::Language::Json), source),
            source
        );
        assert_eq!(format(None, source), source);
    }

    #[test]
    fn formatter_uses_crlf_for_inserted_wraps_in_crlf_input() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three\r\n";
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            "漢 A one\r\ntwo\r\nthree\r\n"
        );
    }

    #[test]
    fn formatter_uses_cr_for_inserted_wraps_in_cr_input_and_preserves_terminators() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three\r漢A one two three\r";
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            "漢 A one\rtwo\rthree\r漢 A one\rtwo\rthree\r"
        );
    }

    #[test]
    fn format_uses_preceding_crlf_for_wraps_in_unterminated_final_line() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three\r\n漢A one two three";

        let formatter = Formatter::new(&config).unwrap();
        assert_eq!(
            formatter
                .format(Some(crate::language::Language::Markdown), source)
                .unwrap(),
            "漢 A one\r\ntwo\r\nthree\r\n漢 A one\r\ntwo\r\nthree"
        );
    }

    #[test]
    fn format_uses_preceding_cr_for_wraps_in_unterminated_final_line() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three\r漢A one two three";

        let formatter = Formatter::new(&config).unwrap();
        assert_eq!(
            formatter
                .format(Some(crate::language::Language::Markdown), source)
                .unwrap(),
            "漢 A one\rtwo\rthree\r漢 A one\rtwo\rthree"
        );
    }

    #[test]
    fn format_uses_preceding_lf_for_wraps_in_unterminated_final_line() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three\n漢A one two three";

        let formatter = Formatter::new(&config).unwrap();
        assert_eq!(
            formatter
                .format(Some(crate::language::Language::Markdown), source)
                .unwrap(),
            "漢 A one\ntwo\nthree\n漢 A one\ntwo\nthree"
        );
    }

    #[test]
    fn format_uses_lf_for_wraps_in_unterminated_single_line_document() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three";

        let formatter = Formatter::new(&config).unwrap();
        assert_eq!(
            formatter
                .format(Some(crate::language::Language::Markdown), source)
                .unwrap(),
            "漢 A one\ntwo\nthree"
        );
    }

    #[test]
    fn formatter_uses_lf_for_inserted_wraps_in_lf_input() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three\n";
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            "漢 A one\ntwo\nthree\n"
        );
    }

    #[test]
    fn markdown_wraps_prose_without_splitting_an_inline_link_destination() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = "[漢A](https://example.test/a-very-long-destination)";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();

        assert_eq!(
            formatted,
            "[漢\nA](https://example.test/a-very-long-destination)"
        );
        assert!(
            !crate::parser::parse(crate::parser::Grammar::Markdown, &formatted)
                .unwrap()
                .root_node()
                .has_error()
        );
    }

    #[test]
    fn markdown_wraps_each_supported_visible_inline_prose_construct() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();

        for source in [
            "*漢漢漢漢漢*",
            "~~漢漢漢漢漢~~",
            "[漢漢漢漢漢](dest)",
            "![漢漢漢漢漢](img)",
        ] {
            let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
            assert!(
                formatted.contains('\n'),
                "did not wrap {source:?}: {formatted:?}"
            );
            assert!(!formatted.contains("\nhttps://"));
        }
    }

    #[test]
    fn markdown_does_not_turn_a_literal_backslash_before_a_wrap_into_a_hard_break() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = r"aaaaaaa\ words more";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            "aaaaaaa\n\\ words\nmore"
        );
    }

    #[test]
    fn markdown_replaces_ordinary_ascii_space_when_wrapping() {
        let mut config = config();
        config.max_width = 4;
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(
            formatter
                .format(Some(Language::Markdown), "aaaa bbbb")
                .unwrap(),
            "aaaa\nbbbb"
        );
    }

    #[test]
    fn markdown_preserves_no_break_characters_adjacent_to_replaceable_spaces() {
        let mut config = config();
        config.max_width = 4;
        let formatter = Formatter::new(&config).unwrap();

        for source in [
            "aaaa\u{00a0} bbbb",
            "aaaa\u{202f} bbbb",
            "aaaa\u{2060} bbbb",
            "aaaa\u{200d} bbbb",
        ] {
            assert_eq!(
                formatter.format(Some(Language::Markdown), source).unwrap(),
                source,
                "wrapped across a no-break character: {source:?}"
            );
        }
    }

    #[test]
    fn markdown_does_not_split_empty_seams_adjacent_to_modified_glue_or_joiners() {
        let mut config = config();
        config.max_width = 2;
        config.spacing.alphabets = SpacingRule::Ignore;
        let formatter = Formatter::new(&config).unwrap();

        for source in [
            "漢\u{00a0}\u{0308}漢",
            "漢\u{2060}\u{0308}漢",
            "漢\u{200d}\u{0308}漢",
            "漢\u{00a0}\u{fe0f}漢",
            "漢\u{2060}\u{fe0f}漢",
            "漢\u{200d}\u{fe0f}漢",
        ] {
            assert_eq!(
                formatter.format(Some(Language::Markdown), source).unwrap(),
                source,
                "wrapped across modified glue or joiner: {source:?}"
            );
        }
    }

    #[test]
    fn markdown_wraps_prose_after_protected_inline_spans() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();

        for (source, node_kind) in [
            ("`very-long-code-span` 漢漢漢漢漢", "code_span"),
            ("$very-long-latex-span$ 漢漢漢漢漢", "latex_block"),
        ] {
            let inline_tree =
                crate::parser::parse(crate::parser::Grammar::MarkdownInline, source).unwrap();
            assert!(has_node_kind(inline_tree.root_node(), node_kind));

            let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
            let span_end = source.find(' ').unwrap();
            assert_eq!(&formatted[..span_end], &source[..span_end]);
            assert_eq!(formatted, format!("{} 漢\n漢漢漢漢", &source[..span_end]));
        }
    }

    #[test]
    fn markdown_wraps_direct_blockquotes_with_their_marker_as_continuation() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = "> aaaa bbbb cccc";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();

        assert_eq!(formatted, "> aaaa\n> bbbb\n> cccc");
        let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();
        assert!(has_node_kind(tree.root_node(), "block_quote"));
        assert_eq!(
            formatter
                .format(Some(Language::Markdown), &formatted)
                .unwrap(),
            formatted
        );
    }

    #[test]
    fn markdown_wraps_top_level_unordered_list_items_with_continuation_indentation() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = "- aaaa bbbb cccc";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();

        assert_eq!(formatted, "- aaaa\n  bbbb\n  cccc");
        let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();
        assert!(has_node_kind(tree.root_node(), "list"));
        assert_eq!(
            formatter
                .format(Some(Language::Markdown), &formatted)
                .unwrap(),
            formatted
        );
    }

    #[test]
    fn markdown_counts_continuation_prefix_width_for_quotes_and_list_items() {
        let mut config = config();
        config.max_width = 6;
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(
            formatter
                .format(Some(Language::Markdown), "> 漢漢漢漢漢")
                .unwrap(),
            "> 漢漢\n> 漢漢\n> 漢"
        );
        assert_eq!(
            formatter
                .format(Some(Language::Markdown), "- 漢漢漢漢漢")
                .unwrap(),
            "- 漢漢\n  漢漢\n  漢"
        );
    }

    #[test]
    fn markdown_keeps_nested_blockquotes_and_lists_ineligible_for_wrapping() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();

        for source in [
            "> > aaaa bbbb cccc",
            "> - aaaa bbbb cccc",
            "- - aaaa bbbb cccc",
        ] {
            assert_eq!(
                formatter.format(Some(Language::Markdown), source).unwrap(),
                source,
                "wrapped unsupported nested context: {source:?}"
            );
        }
    }

    #[test]
    fn markdown_keeps_all_protected_inline_forms_indivisible() {
        let mut config = config();
        config.max_width = 2;
        let formatter = Formatter::new(&config).unwrap();
        for source in [
            "`漢漢漢`",
            "[text](https://example.test/very-long-destination \"long title\")",
            "<https://example.test/very-long-autolink>",
            "<a href=\"very-long-html-attribute\">",
            "&amp;",
            "\\*",
            "[text][very-long-label]",
        ] {
            assert_eq!(
                formatter.format(Some(Language::Markdown), source).unwrap(),
                source,
                "protected Markdown syntax was wrapped: {source:?}"
            );
        }
    }

    #[test]
    fn markdown_does_not_create_setext_or_thematic_breaks_when_wrapping() {
        let mut config = config();
        config.max_width = 5;
        let formatter = Formatter::new(&config).unwrap();

        for delimiter in ["---", "===", "***", "___"] {
            let source = format!("{delimiter} 普通普通普通");
            let formatted = formatter.format(Some(Language::Markdown), &source).unwrap();
            assert!(
                !formatted.starts_with(&format!("{delimiter}\n")),
                "delimiter became a separate physical line: {formatted:?}"
            );
            let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();
            assert!(!tree.root_node().has_error());
            assert!(!has_node_kind(tree.root_node(), "setext_heading"));
            assert!(!has_node_kind(tree.root_node(), "thematic_break"));
        }
    }

    #[test]
    fn markdown_does_not_create_a_thematic_break_from_spaced_delimiters() {
        let mut config = config();
        config.max_width = 5;
        let formatter = Formatter::new(&config).unwrap();
        let source = "_ _ _ foo bar baz";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
        let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();

        assert!(!has_node_kind(tree.root_node(), "thematic_break"));
        assert!(!formatted.starts_with("_ _ _\n"));
    }

    #[test]
    fn markdown_does_not_create_a_pipe_table_from_adjacent_pipe_seam() {
        let mut config = config();
        config.max_width = 12;
        let formatter = Formatter::new(&config).unwrap();
        let source = "heading | | --- | tail words";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
        let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();

        assert_eq!(formatted, source);
        assert!(!has_node_kind(tree.root_node(), "pipe_table"));
    }

    #[test]
    fn markdown_keeps_multiseam_pipe_table_candidates_unchanged() {
        let mut config = config();
        config.max_width = 12;
        let formatter = Formatter::new(&config).unwrap();
        let source = "head | col | --- | --- | tail words";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
        let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();

        assert_eq!(formatted, source);
        assert!(!has_node_kind(tree.root_node(), "pipe_table"));
    }

    #[test]
    fn markdown_wraps_pipe_bearing_prose_without_delimiter_syntax() {
        let mut config = config();
        config.max_width = 12;
        let formatter = Formatter::new(&config).unwrap();
        let source = "normal prose | with trailing words";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
        let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();

        assert!(formatted.contains('\n'));
        assert!(!has_node_kind(tree.root_node(), "pipe_table"));
    }

    #[test]
    fn markdown_does_not_create_a_single_equals_setext_heading_when_wrapping() {
        let mut config = config();
        config.max_width = 5;
        let formatter = Formatter::new(&config).unwrap();
        let source = "heading\n= xxxxx";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
        let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();

        assert_eq!(formatted, source);
        assert!(!has_node_kind(tree.root_node(), "setext_heading"));
    }

    #[test]
    fn markdown_formats_visible_prose_after_an_unclosed_autolink_candidate() {
        let formatter = Formatter::new(&config()).unwrap();
        let source = "漢A <https://example.test/a trailing 漢A";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            "漢 A <https://example.test/a trailing 漢 A"
        );
    }

    #[test]
    fn markdown_preserves_spacing_after_an_unrecognized_html_candidate() {
        let formatter = Formatter::new(&config()).unwrap();
        assert_eq!(
            formatter
                .format(Some(Language::Markdown), "漢A <div 漢A")
                .unwrap(),
            "漢 A <div 漢 A"
        );
    }

    #[test]
    fn markdown_preserves_spacing_inside_multiline_malformed_autolink_candidates() {
        let formatter = Formatter::new(&config()).unwrap();
        let source = "<https://example.test/\n漢A>";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            source
        );
    }

    #[test]
    fn markdown_keeps_raw_html_payloads_indivisible() {
        let mut config = config();
        config.max_width = 5;
        let formatter = Formatter::new(&config).unwrap();
        let source = "漢漢 <script>return 1;</script>";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();

        assert_eq!(formatted, source);
        assert!(!formatted.contains(['\r', '\n']));
    }

    #[test]
    fn markdown_does_not_reinterpret_html_block_candidates_when_wrapping() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();

        for source in [
            "veryvery <script very long raw HTML payload",
            "veryvery <!-- very long comment payload",
            "veryvery <? very long processing instruction",
            "veryvery <!DOCTYPE very long declaration",
            "veryvery <![CDATA[ very long payload",
            "veryvery <div very long block-tag payload",
        ] {
            let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
            assert_eq!(
                formatted, source,
                "HTML-looking prose was reinterpreted: {source:?}"
            );
            let tree = crate::parser::parse(crate::parser::Grammar::Markdown, &formatted).unwrap();
            assert!(!has_node_kind(tree.root_node(), "html_block"));
        }
    }

    #[test]
    fn markdown_wraps_after_a_completed_emoji_zwj_cluster() {
        let mut config = config();
        config.max_width = 4;
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(
            formatter
                .format(Some(Language::Markdown), "👩‍👩 bbbb")
                .unwrap(),
            "👩‍👩\nbbbb"
        );
    }

    #[test]
    fn markdown_does_not_wrap_at_seams_immediately_adjacent_to_a_zero_width_joiner() {
        let mut config = config();
        config.max_width = 2;
        let formatter = Formatter::new(&config).unwrap();

        for source in ["👩‍ bbbb", "👩 \u{200d}bbbb"] {
            assert_eq!(
                formatter.format(Some(Language::Markdown), source).unwrap(),
                source,
                "wrapped at a zero-width joiner seam: {source:?}"
            );
        }
    }

    #[test]
    fn markdown_keeps_an_unclosed_autolink_indivisible() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = "<https://example.test/a-very-long-destination";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            source
        );
    }

    #[test]
    fn markdown_keeps_malformed_uri_and_email_candidates_with_their_trailing_range() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();

        for source in [
            "漢漢漢 <https://example.test/a trailing-prose",
            "漢漢漢 <person@example.test trailing-prose",
            "漢漢漢 <foo:very-long-payload-without-close trailing prose",
        ] {
            let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
            let candidate_start = formatted.find('<').unwrap();
            assert_eq!(
                &formatted[candidate_start..],
                &source[source.find('<').unwrap()..],
                "malformed autolink candidate was split: {source:?}"
            );
            assert!(!formatted[candidate_start..].contains(['\r', '\n']));
        }
    }

    #[test]
    fn markdown_keeps_scheme_autolink_payloads_indivisible() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = "<mailto:verylonglocalpart at example.test>";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            source
        );
    }

    #[test]
    fn markdown_preserves_semantic_two_space_hard_breaks() {
        let mut config = config();
        config.max_width = 2;
        let formatter = Formatter::new(&config).unwrap();
        let source = "漢漢  \n漢漢";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            source
        );
    }

    #[test]
    fn malformed_markdown_document_disables_all_wrapping() {
        let mut config = config();
        config.max_width = 2;
        let formatter = Formatter::new(&config).unwrap();
        let source = "```\n漢漢漢\n";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            source
        );
    }

    #[test]
    fn malformed_inline_range_does_not_disable_wrapping_in_a_sound_range() {
        let mut config = config();
        config.max_width = 4;
        let formatter = Formatter::new(&config).unwrap();
        let source = "`漢漢\n\n漢漢漢";

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            "`漢漢\n\n漢漢\n漢"
        );
    }

    #[test]
    fn formatter_is_idempotent_for_known_language() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = "漢A one two three\r\n";

        let formatted = formatter.format(Some(Language::Markdown), source).unwrap();
        assert_eq!(
            formatter
                .format(Some(Language::Markdown), &formatted)
                .unwrap(),
            formatted
        );
    }

    #[test]
    fn formatter_is_idempotent_for_wrapping_json() {
        let mut config = config();
        config.max_width = 8;
        let formatter = Formatter::new(&config).unwrap();
        let source = r#"{"a":1,"b":2,"c":3}"#;

        let formatted = formatter.format(Some(Language::Json), source).unwrap();
        assert_ne!(formatted, source);
        serde_json::from_str::<serde_json::Value>(&formatted)
            .expect("wrapping must preserve valid JSON");
        assert_eq!(
            formatter.format(Some(Language::Json), &formatted).unwrap(),
            formatted
        );
    }

    #[test]
    fn formatter_preserves_each_source_line_ending_when_wrapping_mixed_input() {
        let mut config = config();
        config.max_width = 8;
        let source = "漢A one two three\r\n漢A one two three\n";
        let formatter = Formatter::new(&config).unwrap();

        assert_eq!(
            formatter.format(Some(Language::Markdown), source).unwrap(),
            "漢 A one\r\ntwo\r\nthree\r\n漢 A one\ntwo\nthree\n"
        );
    }
}
