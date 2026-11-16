mod support;

use std::process::Output;

fn run_cjkfmt(command: &str, input: &str) -> Output {
    support::run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            command,
            "--language",
            "python",
        ],
        input,
    )
}

#[test]
fn python_comments_format_prose_around_inline_code() {
    let source = "# 漢A `漢A` 漢A\nx = 1  # 漢A `漢A` 漢A\n";
    let formatted = run_cjkfmt("format", source);

    assert_eq!(formatted.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(formatted.stdout).unwrap(),
        "# 漢 A `漢A` 漢 A\nx = 1  # 漢 A `漢A` 漢 A\n"
    );
    assert!(formatted.stderr.is_empty());
}

#[test]
fn python_comments_format_link_display_text_and_nearby_prose_but_keep_destinations() {
    let source = "# 漢A https://example.test/漢A 漢A\n# [漢A](https://example.test/漢A) 漢A\nx = 1  # www.example.test/漢A 漢A\n";
    let formatted = run_cjkfmt("format", source);

    assert_eq!(formatted.status.code(), Some(0));
    let formatted_source = String::from_utf8(formatted.stdout).unwrap();
    assert_eq!(
        formatted_source,
        "# 漢 A https://example.test/漢A 漢 A\n# [漢 A](https://example.test/漢A) 漢 A\nx = 1  # www.example.test/漢A 漢 A\n"
    );
    assert!(formatted.stderr.is_empty());
    let second_pass = run_cjkfmt("format", &formatted_source);
    assert_eq!(second_pass.status.code(), Some(0));
    assert_eq!(second_pass.stdout, formatted_source.as_bytes());

    let checked = run_cjkfmt("check", source);
    assert_eq!(checked.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(checked.stdout).unwrap(),
        concat!(
            "<stdin>:1:4: W002 Possible spacing position found\n",
            "<stdin>:1:31: W002 Possible spacing position found\n",
            "<stdin>:2:5: W002 Possible spacing position found\n",
            "<stdin>:2:34: W002 Possible spacing position found\n",
            "<stdin>:3:31: W002 Possible spacing position found\n"
        )
    );
    assert!(checked.stderr.is_empty());
}

#[test]
fn python_link_display_text_remains_formatable_with_an_ipv6_destination() {
    let source = "# [漢A](http://[::1]/漢A) 漢A\n";
    let formatted = run_cjkfmt("format", source);

    assert_eq!(formatted.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(formatted.stdout).unwrap(),
        "# [漢 A](http://[::1]/漢A) 漢 A\n"
    );
    assert!(formatted.stderr.is_empty());
}

#[test]
fn python_standalone_comment_groups_preserve_multiline_inline_code_and_format_both_sides() {
    let source = "# 漢A `` `code`\n# 漢A code `` 漢A\n";
    let formatted = run_cjkfmt("format", source);

    assert_eq!(formatted.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(formatted.stdout).unwrap(),
        "# 漢 A `` `code`\n# 漢A code `` 漢 A\n"
    );
    assert!(formatted.stderr.is_empty());
}

#[test]
fn python_comments_fail_closed_after_an_unclosed_url_autolink_candidate() {
    let source = "# 漢A <https://example.test/漢A 漢A\n\n# 漢A\n";
    let formatted = run_cjkfmt("format", source);

    assert_eq!(formatted.status.code(), Some(0));
    let formatted_source = String::from_utf8(formatted.stdout).unwrap();
    assert_eq!(
        formatted_source,
        "# 漢 A <https://example.test/漢A 漢A\n\n# 漢 A\n"
    );
    assert!(formatted.stderr.is_empty());

    let checked = run_cjkfmt("check", source);
    assert_eq!(checked.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(checked.stdout).unwrap(),
        concat!(
            "<stdin>:1:4: W002 Possible spacing position found\n",
            "<stdin>:3:4: W002 Possible spacing position found\n"
        )
    );
    assert!(checked.stderr.is_empty());
}

#[test]
fn python_comments_stop_formatting_after_incomplete_inline_markup_and_report_only_safe_edits() {
    let source = "# 漢A `unfinished 漢A\n# 漢A\n\n# 漢A [broken](target\n# 漢A\n\n# 漢A\n";
    let formatted = run_cjkfmt("format", source);

    assert_eq!(formatted.status.code(), Some(0));
    let formatted_source = String::from_utf8(formatted.stdout).unwrap();
    assert_eq!(
        formatted_source,
        "# 漢 A `unfinished 漢A\n# 漢A\n\n# 漢 A [broken](target\n# 漢A\n\n# 漢 A\n"
    );
    assert!(formatted.stderr.is_empty());
    let second_pass = run_cjkfmt("format", &formatted_source);
    assert_eq!(second_pass.status.code(), Some(0));
    assert_eq!(second_pass.stdout, formatted_source.as_bytes());

    let checked = run_cjkfmt("check", source);
    assert_eq!(checked.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(checked.stdout).unwrap(),
        concat!(
            "<stdin>:1:4: W002 Possible spacing position found\n",
            "<stdin>:4:4: W002 Possible spacing position found\n",
            "<stdin>:7:4: W002 Possible spacing position found\n"
        )
    );
    assert!(checked.stderr.is_empty());
}
