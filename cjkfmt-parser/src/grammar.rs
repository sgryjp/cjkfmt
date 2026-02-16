use std::path::Path;

/// Supported grammar types for parsing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Grammar {
    Json,
    Markdown,
}

/// Infers the grammar type from the file extension of the given path.
pub fn grammar_from_path<P: AsRef<Path>>(path: P) -> Grammar {
    let path = path.as_ref();
    match path.extension().map(|s| s.to_str().unwrap()) {
        Some("json") => Grammar::Json,
        _ => Grammar::Markdown,
    }
}
