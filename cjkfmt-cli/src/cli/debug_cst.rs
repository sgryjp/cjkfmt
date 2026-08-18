use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use cjkfmt_parser::{Grammar, grammar_from_path, parse};
use tree_sitter::{Node, Tree};

pub fn debug_cst_command<W, P>(stdout: &mut W, filenames: &[P]) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
{
    let mut stdin = stdin();
    debug_cst_command_with_reader(stdout, filenames, &mut stdin)
}

fn debug_cst_command_with_reader<W, P, R>(
    stdout: &mut W,
    filenames: &[P],
    stdin: &mut R,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
    R: Read,
{
    if filenames.is_empty() {
        let mut content = String::with_capacity(1024);
        stdin.read_to_string(&mut content)?;
        write_tree(stdout, Grammar::Markdown, &content)?;
    } else {
        for filename in filenames {
            let filename = filename.as_ref();
            let grammar = grammar_from_path(filename);
            let content = fs::read_to_string(filename)?;
            write_tree(stdout, grammar, &content)?;
        }
    }

    Ok(())
}

fn write_tree<W: std::io::Write>(
    stdout: &mut W,
    grammar: Grammar,
    content: &str,
) -> anyhow::Result<()> {
    let tree = parse(grammar, content)?;
    writeln!(stdout, "{}", render_tree(&tree))?;
    Ok(())
}

fn render_tree(tree: &Tree) -> String {
    let mut output = String::new();
    render_node(&mut output, tree.root_node(), None, 0);
    output
}

fn render_node(output: &mut String, node: Node<'_>, field_name: Option<&str>, indent: usize) {
    output.push_str(&" ".repeat(indent));
    if let Some(field_name) = field_name {
        output.push_str(field_name);
        output.push_str(": ");
    }

    output.push('(');
    output.push_str(node.kind());
    output.push(' ');
    output.push_str(&format_position(node.start_position()));
    output.push_str(" - ");
    output.push_str(&format_position(node.end_position()));

    let named_child_count = node.named_child_count();
    if named_child_count == 0 {
        output.push(')');
        return;
    }

    for i in 0..named_child_count {
        output.push('\n');
        let child = node
            .named_child(i as u32)
            .expect("named child should exist");
        let field_name = node.field_name_for_named_child(i as u32);
        render_node(output, child, field_name, indent + 2);
    }

    output.push(')');
}

fn format_position(point: tree_sitter::Point) -> String {
    format!("[{}, {}]", point.row, point.column)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn make_temp_path(extension: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cjkfmt-debug-cst-{unique}.{extension}"))
    }

    #[test]
    fn debug_cst_command_reads_from_stdin_as_markdown() {
        let mut stdout = Vec::new();
        let mut stdin = "# Test\n".as_bytes();

        debug_cst_command_with_reader(&mut stdout, &[] as &[PathBuf], &mut stdin).unwrap();

        let actual = String::from_utf8(stdout).unwrap();
        assert_eq!(
            actual,
            concat!(
                "(document [0, 0] - [1, 0]\n",
                "  (section [0, 0] - [1, 0]\n",
                "    (atx_heading [0, 0] - [1, 0]\n",
                "      (atx_h1_marker [0, 0] - [0, 1])\n",
                "      heading_content: (inline [0, 2] - [0, 6]))))\n",
            )
        );
    }

    #[test]
    fn debug_cst_command_uses_markdown_grammar_for_uppercase_json_files() {
        let path = make_temp_path("JSON");
        fs::write(&path, "# Test\n").unwrap();

        let mut stdout = Vec::new();
        let mut stdin = "".as_bytes();
        debug_cst_command_with_reader(&mut stdout, &[&path], &mut stdin).unwrap();

        let actual = String::from_utf8(stdout).unwrap();
        assert!(actual.contains("(section [0, 0] - [1, 0]"));
        assert!(!actual.contains("(object "));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn debug_cst_command_uses_json_grammar_for_lowercase_json_files() {
        let path = make_temp_path("json");
        fs::write(&path, "{\"name\":1}\n").unwrap();

        let mut stdout = Vec::new();
        let mut stdin = "".as_bytes();
        debug_cst_command_with_reader(&mut stdout, &[&path], &mut stdin).unwrap();

        let actual = String::from_utf8(stdout).unwrap();
        assert_eq!(
            actual,
            concat!(
                "(document [0, 0] - [1, 0]\n",
                "  (object [0, 0] - [0, 10]\n",
                "    (pair [0, 1] - [0, 9]\n",
                "      key: (string [0, 1] - [0, 7]\n",
                "        (string_content [0, 2] - [0, 6]))\n",
                "      value: (number [0, 8] - [0, 9]))))\n",
            )
        );

        fs::remove_file(path).unwrap();
    }
}
