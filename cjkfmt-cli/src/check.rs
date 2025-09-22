use cjkfmt_core::{diagnostic::Diagnostic, lines_inclusive::LinesInclusiveExt, position::Position};
use cjkfmt_parser::NodeVisitor;
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

    // Make sure the document was already parsed.
    let Some(tree) = document.tree() else {
        anyhow::bail!("the document passed to check_one_file does not have CST.");
    };

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
    let mut spacing_checker = SpacingChecker::new(config, document);
    spacing_checker.walk(tree);
    diagnostics.extend(spacing_checker.diagnostics().iter().cloned());

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
    use cjkfmt_parser::Grammar;

    use super::*;

    #[test]
    fn check_one_file_should_fail_if_called_before_parse() {
        let config = Config::default();
        let document = Document::new::<&str, &str>("# Subject", Grammar::Markdown, None);
        assert!(check_one_file(&config, &document).is_err());
    }
}
