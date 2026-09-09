use crate::parser::FileGrammar;

use crate::core::lines_inclusive::LinesInclusiveExt;
use crate::{
    config::Config,
    line_break::{BreakPoint, LineBreaker},
    markdown_prose::plan_edits,
};

pub(crate) fn format_one_file<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    file_grammar: Option<FileGrammar>,
    content: &str,
) -> Result<(), anyhow::Error> {
    // Keep Markdown spacing selection separate from line wrapping. Both
    // Markdown and non-Markdown inputs retain the existing wrapping pass.
    let content = if file_grammar == Some(FileGrammar::Markdown) {
        let edits = plan_edits(config, content)?;
        let mut content = content.to_owned();
        for edit in edits.into_iter().rev() {
            content.replace_range(edit.range, &edit.replacement);
        }
        content
    } else {
        content.to_owned()
    };

    let line_breaker = LineBreaker::builder()
        .ambiguous_width(config.ambiguous_width)
        .max_width(config.max_width)
        .build()?;

    // Iterate over each line in the input content, including line endings
    for line in content.lines_inclusive() {
        let mut remainings = line;

        // Iterate over wrap points in the line
        while let BreakPoint::WrapPoint {
            overflow_pos,
            adjustment,
        } = line_breaker.next_line_break(remainings)
        {
            // Write the part before the wrap point
            let (before, after) = remainings.split_at(overflow_pos - adjustment);
            writeln!(stdout, "{before}")?;
            remainings = after;
        }

        // Write any remaining part of the line after the last wrap point
        write!(stdout, "{remainings}")?;
    }
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

    fn format(file_grammar: Option<FileGrammar>, source: &str) -> String {
        let mut output = Vec::new();
        format_one_file(&mut output, &config(), file_grammar, source).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn format_applies_configured_spacing_to_markdown_prose() {
        assert_eq!(format(Some(FileGrammar::Markdown), "漢A\n"), "漢 A\n");
    }

    #[test]
    fn format_preserves_spacing_in_non_markdown_input() {
        let source = "{\"value\":\"漢A\"}\n";
        assert_eq!(format(Some(FileGrammar::Json), source), source);
        assert_eq!(format(None, source), source);
    }
}
