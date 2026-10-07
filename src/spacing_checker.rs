use crate::core::{diagnostic::Diagnostic, physical_source::PhysicalSource};
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
    pub fn check(&self, source: &PhysicalSource<'_>) -> anyhow::Result<Vec<Diagnostic>> {
        let edits = plan_spacing_edits(self.language, source, &self.config.spacing)?;
        Ok(edits
            .iter()
            .map(|edit| self.diagnostic_for_edit(source, edit))
            .collect())
    }
}

impl<'a> SpacingChecker<'a> {
    fn diagnostic_for_edit(&self, source: &PhysicalSource<'_>, edit: &TextEdit) -> Diagnostic {
        let text = source.text();
        let absolute_start = edit.range.start;
        let start = source.position(absolute_start);
        let absolute_end = if edit.range.is_empty() {
            absolute_start
                + text[absolute_start..]
                    .graphemes(true)
                    .next()
                    .map_or(0, str::len)
        } else {
            edit.range.end
        };
        let end = source.position(absolute_end);

        Diagnostic::new(
            self.document.filename.as_deref(),
            start,
            end,
            "W002".to_string(),
            "Possible spacing position found".to_string(),
        )
    }
}
