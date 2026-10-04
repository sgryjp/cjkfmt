use crate::language::Language;

/// Supported grammar types for parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    Json,
    Markdown,
    MarkdownInline,
}

impl Language {
    /// Resolve a user-facing language to the grammar used by the parser.
    pub(crate) fn grammar(self) -> Grammar {
        match self {
            Self::Json => Grammar::Json,
            Self::Markdown => Grammar::Markdown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_user_facing_language_resolves_to_its_parser_grammar() {
        assert_eq!(Language::Markdown.grammar(), Grammar::Markdown);
        assert_eq!(Language::Json.grammar(), Grammar::Json);
    }
}
