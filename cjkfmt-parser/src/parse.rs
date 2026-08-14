use tree_sitter::{Parser, Tree};

use crate::Grammar;
use crate::errors::CjkfmtParseError;
use crate::ffi::{tree_sitter_json, tree_sitter_markdown, tree_sitter_markdown_inline};

/// Parses the given content string using the specified grammar and returns a syntax tree.
pub fn parse(grammar: Grammar, content: &str) -> Result<Tree, CjkfmtParseError> {
    // Get TSLanguage object corresponding to the specified grammar.
    let language = unsafe {
        match grammar {
            Grammar::Json => tree_sitter_json(),
            Grammar::Markdown => tree_sitter_markdown(),
            Grammar::MarkdownInline => tree_sitter_markdown_inline(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_inline_grammar_parses_plain_text() {
        let tree = parse(Grammar::MarkdownInline, "漢A").unwrap();
        assert_eq!(tree.root_node().kind(), "inline");
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn markdown_inline_grammar_exposes_code_span() {
        let tree = parse(Grammar::MarkdownInline, "`漢A`").unwrap();
        let root = tree.root_node();
        let code_span = root.named_child(0).expect("code span should be named");
        assert_eq!(code_span.kind(), "code_span");
        assert_eq!(code_span.byte_range(), 0..6);
    }

    #[test]
    fn markdown_inline_grammar_exposes_link_text_and_destination() {
        let source = "[漢A](https://example.test/漢A)";
        let tree = parse(Grammar::MarkdownInline, source).unwrap();
        let link = tree
            .root_node()
            .named_child(0)
            .expect("link should be named");
        assert_eq!(link.kind(), "inline_link");

        let child_kinds: Vec<_> = (0..link.named_child_count() as u32)
            .map(|index| link.named_child(index).unwrap().kind())
            .collect();
        assert_eq!(child_kinds, ["link_text", "link_destination"]);
        assert_eq!(link.named_child(0).unwrap().byte_range(), 1..5);
        assert_eq!(link.named_child(1).unwrap().byte_range(), 7..32);
    }
}
