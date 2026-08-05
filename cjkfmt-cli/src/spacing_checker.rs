use cjkfmt_core::{diagnostic::Diagnostic, position::Position};
use cjkfmt_parser::NodeVisitor;
use unicode_segmentation::UnicodeSegmentation;

use crate::{config::Config, document::Document, spacing::search_possible_spacing_positions};

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

            // Search for possible spacing positions and store them as diagnostics
            for i in search_possible_spacing_positions(self.config, text) {
                let absolute_index = range_start + i;
                let text_before = &self.document.content[..absolute_index];
                let line_index = text_before.chars().filter(|&c| c == '\n').count() as u32;
                let line_start = text_before
                    .rsplit_once('\n')
                    .map(|(_, line)| line)
                    .unwrap_or(text_before);

                // Calculate number of characters from the beginning of the line
                let column_index = line_start.graphemes(true).fold(0u32, |acc, s| {
                    acc + s.encode_utf16().fold(0u32, |acc, _| acc + 1)
                });

                // Get number of Unicode scalar values of the next character
                let next_char_len = text[i..]
                    .graphemes(true)
                    .next()
                    .map(|s| s.encode_utf16().fold(0u32, |acc, _| acc + 1))
                    .unwrap_or(0u32);

                let d = Diagnostic::new(
                    self.document.filename.as_deref(),
                    Position::new(line_index, column_index),
                    Position::new(line_index, column_index + next_char_len),
                    "W002".to_string(),
                    "Possible spacing position found".to_string(),
                );
                self.diagnostics.push(d);
            }
        }
    }

    /// Called when exiting a node in the parse tree. No action needed here.
    fn on_exit(&mut self, _node: &tree_sitter::Node) {}
}
