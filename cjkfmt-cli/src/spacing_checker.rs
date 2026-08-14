use cjkfmt_core::{diagnostic::Diagnostic, position::Position};
use cjkfmt_parser::NodeVisitor;
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    config::Config,
    document::Document,
    spacing::{TextEdit, spacing_edits},
};

/// Checks for possible spacing issues in a document by traversing its parse tree.
#[derive(Debug)]
pub struct SpacingChecker<'a> {
    config: &'a Config,
    document: &'a Document,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> SpacingChecker<'a> {
    /// Creates a new SpacingChecker for the given config and document.
    pub fn new(config: &'a Config, document: &'a Document) -> Self {
        Self {
            config,
            document,
            diagnostics: Vec::new(),
        }
    }

    /// Returns a slice of collected diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Implements the NodeVisitor trait to traverse the parse tree and check for spacing issues.
impl<'a> NodeVisitor for SpacingChecker<'a> {
    fn on_enter(&mut self, node: &tree_sitter::Node) {
        if "inline" == node.kind() {
            // Get the the corresponding text from the document
            let range = node.byte_range();
            let range_start = range.start;
            let text = &self.document.content[range];

            // Convert the shared spacing edits into diagnostics. The checker
            // deliberately retains its existing block-inline scope; Markdown
            // prose filtering belongs to the formatter's Markdown module.
            for edit in spacing_edits(self.config, text) {
                let absolute_start = range_start + edit.range.start;
                let absolute_end = range_start + edit.range.end;
                let diagnostic = self.diagnostic_for_edit(&edit, absolute_start, absolute_end);
                self.diagnostics.push(diagnostic);
            }
        }
    }

    /// Called when exiting a node in the parse tree. No action needed here.
    fn on_exit(&mut self, _node: &tree_sitter::Node) {}
}

impl<'a> SpacingChecker<'a> {
    fn diagnostic_for_edit(
        &self,
        edit: &TextEdit,
        absolute_start: usize,
        absolute_end: usize,
    ) -> Diagnostic {
        let text_before = &self.document.content[..absolute_start];
        let line_index = text_before.chars().filter(|&c| c == '\n').count() as u32;
        let line_start = text_before
            .rsplit_once('\n')
            .map(|(_, line)| line)
            .unwrap_or(text_before);
        let column_index = utf16_len(line_start);

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
