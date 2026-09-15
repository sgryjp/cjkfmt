use crate::core::lines_inclusive::LinesInclusiveExt;
use crate::{
    config::Config,
    formatting::{LanguageFormatError, apply_text_edits},
    language::Language,
    line_break::{BreakPoint, LineBreaker},
    markdown_prose::plan_edits,
};

/// An error produced while constructing or running a [`Formatter`].
#[derive(Debug, thiserror::Error)]
pub(crate) enum FormatError {
    #[error("invalid formatter configuration: {0}")]
    Configuration(#[source] anyhow::Error),

    #[error("language formatting plan is invalid: {0}")]
    Language(#[source] LanguageFormatError),

    #[error("failed to format document: {0}")]
    Formatting(#[source] anyhow::Error),
}

/// Formats one complete document without exposing an intermediate output.
#[derive(Debug)]
pub(crate) struct Formatter {
    config: Config,
    line_breaker: LineBreaker,
}

impl Formatter {
    /// Creates a formatter after validating the configuration it will use.
    pub(crate) fn new(config: &Config) -> Result<Self, FormatError> {
        let line_breaker = LineBreaker::builder()
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
        // Keep Markdown spacing selection separate from line wrapping. Both
        // known languages retain the existing wrapping pass at this stage.
        let content = (if language == Language::Markdown {
            let edits = plan_edits(&self.config, source).map_err(FormatError::Formatting)?;
            apply_text_edits(source, &edits).map_err(FormatError::Language)
        } else {
            Ok(source.to_owned())
        })?;

        let mut formatted = String::with_capacity(content.len());
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
            let mut remaining_line = line;

            while let BreakPoint::WrapPoint {
                overflow_pos,
                adjustment,
            } = self.line_breaker.next_line_break(remaining_line)
            {
                let (before, after) = remaining_line.split_at(overflow_pos - adjustment);
                formatted.push_str(before);
                formatted.push_str(line_ending);
                remaining_line = after;
            }

            formatted.push_str(remaining_line);
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
            "漢 A \r\none two \r\nthree\r\n"
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
            "漢 A \rone two \rthree\r漢 A \rone two \rthree\r"
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
            "漢 A \r\none two \r\nthree\r\n漢 A \r\none two \r\nthree"
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
            "漢 A \rone two \rthree\r漢 A \rone two \rthree"
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
            "漢 A \none two \nthree\n漢 A \none two \nthree"
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
            "漢 A \none two \nthree"
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
            "漢 A \none two \nthree\n"
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
            "漢 A \r\none two \r\nthree\r\n漢 A \none two \nthree\n"
        );
    }
}
