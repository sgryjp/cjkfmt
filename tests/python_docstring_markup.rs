use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

fn run_cjkfmt(command: &str, input: &str) -> Output {
    run_with_options(command, input, &[])
}

fn run_with_options(command: &str, input: &str, options: &[&str]) -> Output {
    let directory = tempdir().expect("temporary working directory should be created");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cjkfmt"))
        .args([
            "--color",
            "never",
            "--spacing-alphabets",
            "require",
            "--max-width",
            "1000",
        ])
        .args(options)
        .args([command, "--language", "python"])
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
fn docstrings_format_safe_prose_around_inline_code_and_markdown_links() {
    let source =
        "\"\"\"Before漢A `code漢A` after漢A and [link漢A](https://example.test/漢A).\"\"\"\n";
    let expected =
        "\"\"\"Before 漢 A `code漢A` after 漢 A and [link 漢 A](https://example.test/漢A).\"\"\"\n";

    let formatted = run_cjkfmt("format", source);
    assert_eq!(formatted.status.code(), Some(0));
    assert_eq!(formatted.stdout, expected.as_bytes());
    assert!(formatted.stderr.is_empty());
    let checked = run_cjkfmt("check", expected);
    assert_eq!(checked.status.code(), Some(0));
    assert!(checked.stdout.is_empty());
}

#[test]
fn standalone_urls_and_code_directives_are_protected() {
    let source = "'''before漢A https://example.test/漢A after漢A\n.. code:: python\n\n    code漢A\n\nend漢A'''\n";
    let expected = "'''before 漢 A https://example.test/漢A after 漢 A\n.. code:: python\n\n    code漢A\n\nend 漢 A'''\n";
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    let checked = run_cjkfmt("check", expected);
    assert_eq!(checked.status.code(), Some(0));
}

#[test]
fn concatenated_literals_recognize_markup_without_spacing_across_boundaries() {
    let source = r#"def docs():
    ("before漢A `code漢" "A` after漢A [text漢" "A](https://example.test/漢A) 漢" "A")
"#;
    let expected = r#"def docs():
    ("before 漢 A `code漢" "A` after 漢 A [text 漢" "A](https://example.test/漢A) 漢" "A")
"#;
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn a_fenced_block_can_start_and_end_in_different_literals() {
    let source = "('''before漢A\n``''' '''`python\ncode漢A\n''' '''```\nafter漢A''')\n";
    let expected = "('''before 漢 A\n``''' '''`python\ncode漢A\n''' '''```\nafter 漢 A''')\n";
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn marked_blocks_and_doctest_output_preserve_examples_between_safe_prose() {
    let source = r#"'''before漢A
```python
code漢A
```
between漢A
.. code-block:: python

    code漢A

after漢A
Example::

    literal漢A

more漢A
>>> input漢A
... continuation漢A
output漢A

end漢A'''
"#;
    let expected = r#"'''before 漢 A
```python
code漢A
```
between 漢 A
.. code-block:: python

    code漢A

after 漢 A
Example::

    literal漢A

more 漢 A
>>> input漢A
... continuation漢A
output漢A

end 漢 A'''
"#;
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
    let checked = run_cjkfmt("check", expected);
    assert_eq!(checked.status.code(), Some(0));
    assert!(checked.stdout.is_empty());
}

#[test]
fn incomplete_markup_leaves_the_remainder_but_not_other_docstrings_unchanged() {
    let source =
        "\"\"\"before漢A `unfinished漢A after漢A\"\"\"\ndef f():\n    \"\"\"safe漢A\"\"\"\n";
    let expected =
        "\"\"\"before 漢 A `unfinished漢A after漢A\"\"\"\ndef f():\n    \"\"\"safe 漢 A\"\"\"\n";
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn incomplete_fences_and_links_leave_only_their_remainders_unchanged() {
    for (source, expected) in [
        (
            "'''before漢A\n~~~python\ncode漢A\nafter漢A'''\n",
            "'''before 漢 A\n~~~python\ncode漢A\nafter漢A'''\n",
        ),
        (
            "'''before漢A [link漢A](https://example.test/漢A after漢A'''\n",
            "'''before 漢 A [link漢A](https://example.test/漢A after漢A'''\n",
        ),
    ] {
        let formatted = run_cjkfmt("format", source);
        assert_eq!(formatted.status.code(), Some(0));
        assert_eq!(formatted.stdout, expected.as_bytes());
        let checked = run_cjkfmt("check", expected);
        assert_eq!(checked.status.code(), Some(0));
    }
}

#[test]
fn escaped_newlines_do_not_expose_doctest_examples_to_spacing() {
    let source = r#""before漢A\n>>> example漢A\noutput漢A\n\nafter漢A"
"#;
    let expected = r#""before 漢 A\n>>> example漢A\noutput漢A\n\nafter 漢 A"
"#;
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn raw_docstring_escape_spellings_remain_spacing_boundaries() {
    let source = r#"r"before漢A\n漢A `code漢A` after漢A"
"#;
    let expected = r#"r"before 漢 A\n漢 A `code漢A` after 漢 A"
"#;
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn escaped_crlf_keeps_doctest_output_in_the_example() {
    let source = r#""before漢A\r\n>>> example漢A\r\noutput漢A\r\n\r\nafter漢A"
"#;
    let expected = r#""before 漢 A\r\n>>> example漢A\r\noutput漢A\r\n\r\nafter 漢 A"
"#;
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn escaped_indentation_before_a_doctest_still_protects_its_output() {
    let source = r#""before漢A\n\t>>> example漢A\noutput漢A\n\nafter漢A"
"#;
    let expected = r#""before 漢 A\n\t>>> example漢A\noutput漢A\n\nafter 漢 A"
"#;
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, expected.as_bytes());
}

#[test]
fn encoded_markup_in_concatenated_literals_falls_back_without_corrupting_examples() {
    let source = r#"("before漢A \x60code漢" "A\x60 after漢A")
"#;
    let output = run_cjkfmt("format", source);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, source.as_bytes());
    assert!(output.stderr.is_empty());
}

#[test]
fn check_reports_original_positions_and_both_wrapping_policies_are_idempotent() {
    let source = "\"\"\"漢A `code漢A` 漢B\"\"\"\r\n";
    let expected = "\"\"\"漢 A `code漢A` 漢 B\"\"\"\r\n";
    let checked = run_cjkfmt("check", source);
    assert_eq!(checked.status.code(), Some(1));
    assert_eq!(checked.stdout, b"<stdin>:1:5: W002 Possible spacing position found\n<stdin>:1:17: W002 Possible spacing position found\n");
    assert!(checked.stderr.is_empty());
    for wrapping in ["preserve", "split"] {
        let formatted = run_with_options("format", source, &["--line-wrapping", wrapping]);
        assert_eq!(formatted.status.code(), Some(0));
        assert_eq!(formatted.stdout, expected.as_bytes());
        let formatted_again = run_with_options("format", expected, &["--line-wrapping", wrapping]);
        assert_eq!(formatted_again.status.code(), Some(0));
        assert_eq!(formatted_again.stdout, expected.as_bytes());
    }
}
