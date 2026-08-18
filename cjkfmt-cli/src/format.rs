use crate::{
    config::Config,
    line_break::{BreakPoint, LineBreaker},
    markdown_spacing::apply_markdown_spacing,
};
use cjkfmt_core::lines_inclusive::LinesInclusiveExt;

pub(crate) fn format_one_file<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    apply_spacing: bool,
    content: &str,
) -> Result<(), anyhow::Error> {
    // Keep Markdown spacing selection separate from line wrapping. Both
    // Markdown and non-Markdown inputs retain the existing wrapping pass.
    let content = if apply_spacing {
        apply_markdown_spacing(config, content)?
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

    fn format(apply_markdown_spacing: bool, source: &str) -> String {
        let mut output = Vec::new();
        format_one_file(&mut output, &config(), apply_markdown_spacing, source).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn format_applies_configured_spacing_to_markdown_prose() {
        assert_eq!(format(true, "漢A\n"), "漢 A\n");
    }

    #[test]
    fn format_preserves_spacing_in_non_markdown_input() {
        let source = "{\"value\":\"漢A\"}\n";
        assert_eq!(format(false, source), source);
    }
}
