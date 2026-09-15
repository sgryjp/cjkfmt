use crate::core::{diagnostic::Diagnostic, position::Position};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    config::Config,
    document::Document,
    formatting::{TextEdit, plan_spacing_edits},
    language::Language,
};

/// Checks for possible spacing issues from the selected policy's validated
/// edit plan, which is the same plan used by the formatter.
#[derive(Debug)]
pub struct SpacingChecker<'a> {
    config: &'a Config,
    document: &'a Document,
    language: Language,
}

impl<'a> SpacingChecker<'a> {
    /// Creates a spacing checker for the selected language and document.
    pub fn new(config: &'a Config, document: &'a Document, language: Language) -> Self {
        Self {
            config,
            document,
            language,
        }
    }

    /// Plans spacing edits and converts them to diagnostics.
    pub fn check(&self) -> anyhow::Result<Vec<Diagnostic>> {
        let edits =
            plan_spacing_edits(self.language, &self.document.content, &self.config.spacing)?;
        Ok(edits
            .iter()
            .map(|edit| self.diagnostic_for_edit(edit))
            .collect())
    }
}

impl<'a> SpacingChecker<'a> {
    fn diagnostic_for_edit(&self, edit: &TextEdit) -> Diagnostic {
        let absolute_start = edit.range.start;
        let absolute_end = edit.range.end;
        let text_before = &self.document.content[..absolute_start];
        let line_index = text_before.chars().filter(|&c| c == '\n').count() as u32;
        let line_start = text_before.rfind('\n').map_or(0, |index| index + 1);
        let column_index = utf16_len(&self.document.content[line_start..absolute_start]);

        let end_column = if edit.range.is_empty() {
            self.document.content[absolute_start..]
                .graphemes(true)
                .next()
                .map_or(column_index, |grapheme| column_index + utf16_len(grapheme))
        } else {
            column_index + utf16_len(&self.document.content[absolute_start..absolute_end])
        };

        Diagnostic::new(
            self.document.filename.as_deref(),
            Position::new(line_index, column_index),
            Position::new(line_index, end_column),
            "W002".to_string(),
            "Possible spacing position found".to_string(),
        )
    }
}

fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}
