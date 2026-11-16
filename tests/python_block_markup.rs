use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

fn run_cjkfmt(args: &[&str], input: &str) -> Output {
    let directory = tempdir().expect("temporary working directory should be created");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cjkfmt"))
        .args(["--color", "never"])
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
        .expect("stdin should be written");
    child.wait_with_output().expect("cjkfmt should finish")
}

fn format_python(source: &str) -> String {
    let formatted = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
    );
    assert_eq!(formatted.status.code(), Some(0));
    assert!(formatted.stderr.is_empty());
    String::from_utf8(formatted.stdout).unwrap()
}

fn check_python(source: &str) -> String {
    let checked = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "check",
            "--language",
            "python",
        ],
        source,
    );
    assert_eq!(checked.status.code(), Some(1));
    assert!(checked.stderr.is_empty());
    String::from_utf8(checked.stdout).unwrap()
}

#[test]
fn formats_prose_around_fenced_examples_and_check_reports_only_safe_lines() {
    let source = "# 漢A\n# ```python\n# print(\"漢A\")\n# ```\n# 漢A\n";
    let expected = "# 漢 A\n# ```python\n# print(\"漢A\")\n# ```\n# 漢 A\n";

    assert_eq!(
        format_python(source),
        expected,
        "format should preserve the fenced example while spacing neighboring prose"
    );
    assert_eq!(
        check_python(source),
        "<stdin>:1:4: W002 Possible spacing position found\n\
         <stdin>:5:4: W002 Possible spacing position found\n",
        "check should report only prose outside the fenced example"
    );
    assert_eq!(
        format_python(expected),
        expected,
        "formatting protected examples should be idempotent"
    );
}

#[test]
fn inline_markup_after_a_fence_does_not_block_safe_surrounding_prose() {
    let source = "# ```python\n# print(\"漢A\")\n# ```\n# 漢A `漢A` 漢A\n";
    assert_eq!(
        format_python(source),
        "# ```python\n# print(\"漢A\")\n# ```\n# 漢 A `漢A` 漢 A\n"
    );
}

#[test]
fn protects_indented_content_of_restructuredtext_code_directives() {
    for directive in ["code", "code-block"] {
        let source = format!("# 漢A\n# .. {directive}:: python\n#\n#     print(\"漢A\")\n# 漢A\n");
        let expected =
            format!("# 漢 A\n# .. {directive}:: python\n#\n#     print(\"漢A\")\n# 漢 A\n");
        assert_eq!(format_python(&source), expected, "directive={directive}");
        assert_eq!(
            check_python(&source),
            "<stdin>:1:4: W002 Possible spacing position found\n\
             <stdin>:5:4: W002 Possible spacing position found\n",
            "directive={directive}"
        );
    }
}

#[test]
fn protects_indented_content_of_restructuredtext_literal_blocks() {
    let source = "# Example::\n#\n#     print(\"漢A\")\n# 漢A\n";
    assert_eq!(
        format_python(source),
        "# Example::\n#\n#     print(\"漢A\")\n# 漢 A\n"
    );
    assert_eq!(
        check_python(source),
        "<stdin>:4:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn protects_doctest_prompts_continuations_and_output_until_a_blank_line() {
    let source = "# 漢A\n# >>> print(\"漢A\")\n# ... print(\"漢A\")\n# 漢A\n#\n# 漢A\n";
    assert_eq!(
        format_python(source),
        "# 漢 A\n# >>> print(\"漢A\")\n# ... print(\"漢A\")\n# 漢A\n#\n# 漢 A\n"
    );
    assert_eq!(
        check_python(source),
        "<stdin>:1:4: W002 Possible spacing position found\n\
         <stdin>:6:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn incomplete_fenced_blocks_protect_the_remainder_of_the_comment_group() {
    let source = "# 漢A\n# ```python\n# 漢A\n\n# 漢A\n";
    let expected = "# 漢 A\n# ```python\n# 漢A\n\n# 漢 A\n";
    assert_eq!(format_python(source), expected);
    assert_eq!(
        check_python(source),
        "<stdin>:1:4: W002 Possible spacing position found\n\
         <stdin>:5:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn malformed_fence_openers_fail_closed_for_the_rest_of_the_comment_group() {
    let source = "# 漢A\n# ```invalid`info\n# print(\"漢A\")\n";
    assert_eq!(
        format_python(source),
        "# 漢 A\n# ```invalid`info\n# print(\"漢A\")\n"
    );
}

#[test]
fn incomplete_restructuredtext_directives_fail_closed_through_the_comment_region() {
    let source = "# 漢A\n# .. code-block:: python\n# 漢A\n";
    assert_eq!(
        format_python(source),
        "# 漢 A\n# .. code-block:: python\n# 漢A\n"
    );
    assert_eq!(
        check_python(source),
        "<stdin>:1:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn tilde_fences_require_a_matching_marker_and_sufficient_closing_length() {
    let source = "# 漢A\n# ~~~~python\n# ~~~\n# 漢A\n# ~~~~\n# 漢A\n";
    assert_eq!(
        format_python(source),
        "# 漢 A\n# ~~~~python\n# ~~~\n# 漢A\n# ~~~~\n# 漢 A\n"
    );
}

#[test]
fn unsupported_incomplete_markup_protects_the_remaining_comment_region() {
    let source = "# 漢A\n# [漢A\n# 漢A\n";
    assert_eq!(format_python(source), "# 漢 A\n# [漢A\n# 漢A\n");
    assert_eq!(
        check_python(source),
        "<stdin>:1:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn unmarked_examples_and_machine_directed_comments_remain_ordinary_prose() {
    let source = "# print(\"漢A\")\n# noqa: F821 漢A\n";
    assert_eq!(
        format_python(source),
        "# print(\"漢 A\")\n# noqa: F821 漢 A\n"
    );
    assert_eq!(
        check_python(source),
        "<stdin>:1:11: W002 Possible spacing position found\n\
         <stdin>:2:15: W002 Possible spacing position found\n"
    );
}

#[test]
fn comment_group_boundaries_keep_trailing_and_differently_indented_comments_independent() {
    let source = "value = 1  # ```python\n  # 漢A\n# 漢A\n";
    assert_eq!(
        format_python(source),
        "value = 1  # ```python\n  # 漢 A\n# 漢 A\n"
    );
    assert_eq!(
        check_python(source),
        "<stdin>:2:6: W002 Possible spacing position found\n\
         <stdin>:3:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn comment_markup_protection_preserves_original_line_endings() {
    let source = "# 漢A\r\n# ```python\r\n# print(\"漢A\")\r\n# ```\r\n# 漢A\r\n";
    assert_eq!(
        format_python(source),
        "# 漢 A\r\n# ```python\r\n# print(\"漢A\")\r\n# ```\r\n# 漢 A\r\n"
    );
}
