use tree_sitter::Language;

unsafe extern "C" {
    pub fn tree_sitter_json() -> Language;
    pub fn tree_sitter_markdown() -> Language;
}
