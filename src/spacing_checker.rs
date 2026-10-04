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
        let start = Position::from_offset(&self.document.content, absolute_start);
        let absolute_end = if edit.range.is_empty() {
            absolute_start
                + self.document.content[absolute_start..]
                    .graphemes(true)
                    .next()
                    .map_or(0, str::len)
        } else {
            edit.range.end
        };
        let end = Position::from_offset(&self.document.content, absolute_end);

        Diagnostic::new(
            self.document.filename.as_deref(),
            start,
            end,
            "W002".to_string(),
            "Possible spacing position found".to_string(),
        )
    }
}
