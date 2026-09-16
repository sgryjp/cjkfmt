//! The built-in Markdown formatting policy.

use super::{BreakOpportunity, LanguageFormatError, LanguageFormatPolicy, SpacingRules, TextEdit};
use crate::markdown_prose;

/// Plans Markdown prose spacing and conservative soft-break opportunities.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct MarkdownFormatPolicy;

impl LanguageFormatPolicy for MarkdownFormatPolicy {
    fn plan_spacing_edits(
        &self,
        source: &str,
        rules: &SpacingRules,
    ) -> Result<Vec<TextEdit>, LanguageFormatError> {
        markdown_prose::plan_spacing_edits(source, rules)
    }

    fn plan_break_opportunities(
        &self,
        source: &str,
    ) -> Result<Vec<BreakOpportunity>, LanguageFormatError> {
        markdown_prose::plan_break_opportunities(source)
    }
}
