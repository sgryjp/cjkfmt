use tree_sitter::{Node, Tree};

/// A trait for visiting nodes in a concrete syntax tree built by tree-sitter parsers.
///
/// Implementors of this trait can define custom behavior to execute when entering
/// (`on_enter`) and exiting (`on_exit`) nodes during traversal.
/// This trait has a default implementation of `walk` method which traverses the tree
/// depth-first order.
pub trait NodeVisitor {
    /// Called when entering a node during traversal.
    fn on_enter(&mut self, node: &Node);

    /// Called when exiting a node during traversal.
    fn on_exit(&mut self, node: &Node);

    /// Walks the syntax tree in depth-first order, calling `on_enter` and `on_exit`
    /// at appropriate times for each node.
    fn walk(&mut self, tree: &Tree) {
        let mut cursor = tree.walk();
        'outer: loop {
            self.on_enter(&cursor.node());

            // Select child node
            if cursor.goto_first_child() {
                continue;
            }

            // Select sibling node
            let prev_node = cursor.node();
            if cursor.goto_next_sibling() {
                self.on_exit(&prev_node);
                continue;
            }

            // Search for a parent node that has a sibling node and select it
            loop {
                self.on_exit(&cursor.node());
                if !cursor.goto_parent() {
                    break 'outer; // Reached the root node
                }

                let prev_node = cursor.node();
                if cursor.goto_next_sibling() {
                    self.on_exit(&prev_node);
                    continue 'outer;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tree_sitter::Node;

    use crate::{Grammar, node_visitor::NodeVisitor, parse};

    #[test]
    fn test_visitor() {
        struct MyVisitor<'a> {
            depth: usize,
            source: &'a str,
            pub buffer: String,
        }

        impl<'a> MyVisitor<'a> {
            fn new(source: &'a str) -> Self {
                MyVisitor {
                    depth: 0,
                    source,
                    buffer: String::new(),
                }
            }
        }

        impl<'a> NodeVisitor for MyVisitor<'a> {
            fn on_enter(&mut self, node: &Node) {
                let indent = "  ".repeat(self.depth);
                let range = node.byte_range();
                let substr = self.source.get(range.start..range.end).unwrap();
                match node.kind() {
                    "string_content" | "number" => {
                        self.buffer
                            .push_str(&format!("{indent}> {} {:?}\n", node.kind(), substr));
                    }
                    _ => {
                        self.buffer
                            .push_str(&format!("{indent}> {}\n", node.kind()));
                    }
                }

                self.depth += 1;
            }

            fn on_exit(&mut self, node: &Node) {
                self.depth -= 1;

                let indent = "  ".repeat(self.depth);
                let range = node.byte_range();
                let substr = self.source.get(range.start..range.end).unwrap();
                match node.kind() {
                    "string_content" | "number" => {
                        self.buffer
                            .push_str(&format!("{indent}< {} {:?}\n", node.kind(), substr));
                    }
                    _ => {
                        self.buffer
                            .push_str(&format!("{indent}< {}\n", node.kind()));
                    }
                }
            }
        }

        let content = r#"{"name": "Alice", "published": 1865}"#;
        let tree = parse(Grammar::Json, content).expect("Failed to parse JSON");

        // Example usage of the visitor
        let mut visitor = MyVisitor::new(content);
        visitor.walk(&tree);
        let expected = vec![
            r#"> document"#,
            r#"  > object"#,
            r#"    > {"#,
            r#"    < {"#,
            r#"    > pair"#,
            r#"      > string"#,
            r#"        > ""#,
            r#"        < ""#,
            r#"        > string_content "name""#,
            r#"        < string_content "name""#,
            r#"        > ""#,
            r#"        < ""#,
            r#"      < string"#,
            r#"      > :"#,
            r#"      < :"#,
            r#"      > string"#,
            r#"        > ""#,
            r#"        < ""#,
            r#"        > string_content "Alice""#,
            r#"        < string_content "Alice""#,
            r#"        > ""#,
            r#"        < ""#,
            r#"      < string"#,
            r#"    < pair"#,
            r#"    > ,"#,
            r#"    < ,"#,
            r#"    > pair"#,
            r#"      > string"#,
            r#"        > ""#,
            r#"        < ""#,
            r#"        > string_content "published""#,
            r#"        < string_content "published""#,
            r#"        > ""#,
            r#"        < ""#,
            r#"      < string"#,
            r#"      > :"#,
            r#"      < :"#,
            r#"      > number "1865""#,
            r#"      < number "1865""#,
            r#"    < pair"#,
            r#"    > }"#,
            r#"    < }"#,
            r#"  < object"#,
            r#"< document"#,
        ];
        let trace_log = visitor.buffer.as_str();
        let num_different: Vec<Option<usize>> = trace_log
            .lines()
            .zip(&expected)
            .enumerate()
            .map(|(i, (actual, expected))| {
                if actual == *expected {
                    eprintln!("{:3} {}", i, actual);
                    None
                } else {
                    eprintln!("{:3}-{}", i, actual);
                    eprintln!("{:3}+{}", i, expected);
                    Some(i)
                }
            })
            .collect();
        assert!(
            num_different.iter().all(|n| n.is_none()),
            "UNMATCHED LINES: {:?}",
            num_different.into_iter().flatten().collect::<Vec<usize>>(),
        );
    }
}
