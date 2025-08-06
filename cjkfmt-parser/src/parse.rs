use tree_sitter::{Parser, Tree};

use crate::errors::CjkfmtParseError;
use crate::ffi::{tree_sitter_json, tree_sitter_markdown};

/// Supported grammar types for parsing.
pub enum Grammar {
    Json,
    Markdown,
}

/// Parses the given content string using the specified grammar and returns a syntax tree.
pub fn parse(grammar: Grammar, content: &str) -> Result<Tree, CjkfmtParseError> {
    // Get TSLanguage object corresponding to the specified grammar.
    let language = unsafe {
        match grammar {
            Grammar::Json => tree_sitter_json(),
            Grammar::Markdown => tree_sitter_markdown(),
        }
    };

    // Parse the specified content into a concrete syntax tree.
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| CjkfmtParseError::ParseError("failed to parse".to_string()))?;

    Ok(tree)
}
