use crate::language::Language;

/// Supported grammar types for parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    Json,
    Markdown,
    MarkdownInline,
}

/// Convert the canonical language selection at the parser boundary.
pub(crate) fn grammar_for(language: Language) -> Grammar {
    match language {
        Language::Json => Grammar::Json,
        Language::Markdown => Grammar::Markdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_each_language_to_the_parser_grammar_used_at_the_boundary() {
        assert_eq!(grammar_for(Language::Markdown), Grammar::Markdown);
        assert_eq!(grammar_for(Language::Json), Grammar::Json);
    }
}
