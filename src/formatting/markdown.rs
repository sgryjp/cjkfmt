//! The built-in Markdown formatting policy.

use super::{BreakOpportunity, LanguageFormatError, LanguageFormatPolicy, SpacingRules, TextEdit};
use crate::{core::physical_source::PhysicalSource, markdown_prose};

/// Plans Markdown prose spacing and conservative soft-break opportunities.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct MarkdownFormatPolicy;

impl LanguageFormatPolicy for MarkdownFormatPolicy {
    fn plan_spacing_edits(
        &self,
        source: &PhysicalSource<'_>,
        rules: &SpacingRules,
    ) -> Result<Vec<TextEdit>, LanguageFormatError> {
        markdown_prose::plan_spacing_edits_in(source, rules)
    }

    fn plan_break_opportunities(
        &self,
        source: &PhysicalSource<'_>,
    ) -> Result<Vec<BreakOpportunity>, LanguageFormatError> {
        markdown_prose::plan_break_opportunities_in(source)
    }
}
