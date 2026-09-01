use crate::{
    config::Config,
    formatting_ranges::{apply_range_spacing, formatting_ranges},
    line_break::{BreakPoint, LineBreaker},
    markdown_spacing::apply_markdown_spacing,
};
use cjkfmt_core::lines_inclusive::LinesInclusiveExt;
use cjkfmt_parser::{Grammar, parse};

pub(crate) fn format_one_file<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    grammar: Grammar,
    apply_spacing: bool,
    content: &str,
) -> Result<(), anyhow::Error> {
    let content = match grammar {
        Grammar::Markdown if apply_spacing => apply_markdown_spacing(config, content)?,
        Grammar::Markdown => content.to_owned(),
        Grammar::Python => {
            let tree = parse(Grammar::Python, content)?;
            let ranges = formatting_ranges(Grammar::Python, tree.root_node(), content)?;
            apply_range_spacing(config, content, &ranges)?
        }
        _ => content.to_owned(),
    };
    if grammar == Grammar::Python {
        write!(stdout, "{content}")?;
        return Ok(());
    }

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
        let grammar = if apply_markdown_spacing {
            Grammar::Markdown
        } else {
            Grammar::Json
        };
        format_one_file(
            &mut output,
            &config(),
            grammar,
            apply_markdown_spacing,
            source,
        )
        .unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn format_applies_configured_spacing_to_markdown_prose() {
        assert_eq!(format(true, "漢A\n"), "漢 A\n");
    }

    #[test]
    fn python_format_does_not_wrap_code_or_selected_content() {
        let mut config = config();
        config.max_width = 5;
        let source = "# 漢A very long comment\ndef f():\n    \"漢A very long docstring\"\n    return \"漢A\"\n";
        let mut output = Vec::new();
        format_one_file(&mut output, &config, Grammar::Python, true, source).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "# 漢 A very long comment\ndef f():\n    \"漢 A very long docstring\"\n    return \"漢A\"\n"
        );
    }

    #[test]
    fn format_preserves_spacing_in_non_markdown_input() {
        let source = "{\"value\":\"漢A\"}\n";
        assert_eq!(format(false, source), source);
    }
}
