use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

fn run_cjkfmt(args: &[&str], input: &str) -> Output {
    let directory = tempdir().expect("temporary working directory should be created");
    let mut command = Command::new(env!("CARGO_BIN_EXE_cjkfmt"));
    command.args(["--color", "never"]);
    if !args.contains(&"--spacing-alphabets") {
        command.args(["--spacing-alphabets", "require"]);
    }
    let mut child = command
        .args(args)
        .current_dir(directory.path())
        .env_clear()
        .env("HOME", directory.path())
        .env("XDG_CONFIG_HOME", directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cjkfmt should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("input should be written");
    child.wait_with_output().expect("cjkfmt should finish")
}

#[test]
fn actual_module_docstring_spacing_is_formatted_and_reported_at_its_source_position() {
    let source = "\"\"\"漢A\"\"\"\n";
    let formatted = run_cjkfmt(&["format", "--language", "python"], source);
    assert_eq!(formatted.status.code(), Some(0));
    assert_eq!(formatted.stdout, "\"\"\"漢 A\"\"\"\n".as_bytes());
    assert!(formatted.stderr.is_empty());

    let checked = run_cjkfmt(&["check", "--language", "python"], source);
    assert_eq!(checked.status.code(), Some(1));
    assert_eq!(
        checked.stdout,
        b"<stdin>:1:5: W002 Possible spacing position found\n"
    );
    assert!(checked.stderr.is_empty());
}

#[test]
fn class_nested_and_async_function_docstrings_are_selected_by_statement_position() {
    let source = r#"# A leading comment does not count as a statement
class Example:
    # Comments before a docstring do not count as statements
    """class漢A"""
    def nested(self):
        """function漢A"""
        "additional 漢A"
        return "ordinary 漢A"
    async def run(self):
        """async漢A"""
"#;
    let expected = r#"# A leading comment does not count as a statement
class Example:
    # Comments before a docstring do not count as statements
    """class 漢 A"""
    def nested(self):
        """function 漢 A"""
        "additional 漢A"
        return "ordinary 漢A"
    async def run(self):
        """async 漢 A"""
"#;
    let formatted = run_cjkfmt(&["format", "--language", "python"], source);
    assert_eq!(formatted.status.code(), Some(0));
    assert_eq!(formatted.stdout, expected.as_bytes());
    assert!(formatted.stderr.is_empty());
}

#[test]
fn parenthesized_concatenated_raw_and_unicode_literals_preserve_escape_boundaries() {
    let source = r#"def docs():
    ((r"漢A" "漢" "A" u"漢2" "漢A\n漢A"))
"#;
    let expected = r#"def docs():
    ((r"漢 A" "漢" "A" u"漢 2" "漢 A\n漢 A"))
"#;
    let formatted = run_cjkfmt(
        &[
            "--spacing-digits",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
    );
    assert_eq!(formatted.status.code(), Some(0));
    assert_eq!(formatted.stdout, expected.as_bytes());
    assert!(formatted.stderr.is_empty());
}

#[test]
fn ordinary_byte_formatted_and_binary_expression_strings_are_not_docstrings() {
    let source = r#"ordinary = "漢A"
bytes_value = b"漢A"
formatted_value = f"漢A"

def byte_string_expression():
    b"漢A"

def formatted_string_expression():
    f"漢A"

def binary_expression():
    ("left漢A" + "right漢A")

def additional_string():
    value = 1
    "not a docstring漢A"

class AttributeString:
    value = 1
    "attribute string漢A"
"#;
    let output = run_cjkfmt(&["format", "--language", "python"], source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, source.as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn docstrings_with_possible_protected_markup_are_skipped_independently() {
    let source = r##""""module `code漢A` and https://example.test/漢A"""

def safe():
    """safe漢A"""
"##;
    let expected = r##""""module `code漢A` and https://example.test/漢A"""

def safe():
    """safe 漢 A"""
"##;
    let output = run_cjkfmt(&["format", "--language", "python"], source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn docstrings_apply_alphabet_and_digit_spacing_modes_without_wrapping() {
    let source = "\"\"\"漢A 漢 2 漢A 漢 2\"\"\"\n";
    for (rule, expected) in [
        ("require", "\"\"\"漢 A 漢 2 漢 A 漢 2\"\"\"\n"),
        ("prohibit", "\"\"\"漢A漢2漢A漢2\"\"\"\n"),
        ("ignore", source),
    ] {
        let output = run_cjkfmt(
            &[
                "--spacing-alphabets",
                rule,
                "--spacing-digits",
                rule,
                "--line-wrapping",
                "split",
                "--max-width",
                "2",
                "format",
                "--language",
                "python",
            ],
            source,
        );
        assert_eq!(output.status.code(), Some(0), "rule={rule}");
        assert_eq!(output.stdout, expected.as_bytes(), "rule={rule}");
        assert!(output.stderr.is_empty(), "rule={rule}");
    }
}

#[test]
fn write_preserves_docstring_bom_and_line_endings_and_is_idempotent() {
    let directory = tempdir().expect("temporary directory should be created");
    let path = directory.path().join("document.Py");
    let source = "\u{feff}\"\"\"漢A\"\"\"\r\n";
    std::fs::write(&path, source.as_bytes()).expect("Python source should be written");

    for _ in 0..2 {
        let output = Command::new(env!("CARGO_BIN_EXE_cjkfmt"))
            .args([
                "--color",
                "never",
                "--spacing-alphabets",
                "require",
                "format",
                "--write",
            ])
            .arg(&path)
            .current_dir(directory.path())
            .env_clear()
            .env("HOME", directory.path())
            .env("XDG_CONFIG_HOME", directory.path())
            .output()
            .expect("cjkfmt should run");
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    assert_eq!(
        std::fs::read(&path).expect("formatted source should be readable"),
        "\u{feff}\"\"\"漢 A\"\"\"\r\n".as_bytes()
    );
}

#[test]
fn multiline_docstrings_preserve_mixed_physical_line_endings() {
    let source = "'''description漢A\r\nsecond漢2\rthird漢A'''\n";
    let expected = "'''description 漢 A\r\nsecond 漢2\rthird 漢 A'''\n";
    let output = run_cjkfmt(
        &["--line-wrapping", "split", "format", "--language", "python"],
        source,
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
}
