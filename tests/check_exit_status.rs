use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

fn run_cjkfmt(args: &[&str], input: &str, config: Option<&str>) -> Output {
    run_cjkfmt_with_input(args, input.as_bytes(), config, &[])
}

fn run_cjkfmt_in_directory(args: &[&str], directory: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cjkfmt"))
        .args(["--color", "never"])
        .args(args)
        .current_dir(directory)
        .env_clear()
        .env("HOME", directory)
        .env("XDG_CONFIG_HOME", directory)
        .output()
        .expect("cjkfmt should finish")
}

fn run_cjkfmt_with_input(
    args: &[&str],
    input: &[u8],
    config: Option<&str>,
    files: &[(&str, &str)],
) -> Output {
    let directory = tempdir().expect("temporary working directory should be created");
    if let Some(config) = config {
        std::fs::write(directory.path().join(".cjkfmt.json"), config)
            .expect("configuration should be written");
    }
    for (name, contents) in files {
        std::fs::write(directory.path().join(name), contents)
            .expect("input file should be written");
    }

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
        .write_all(input)
        .expect("stdin should be written");

    child.wait_with_output().expect("cjkfmt should finish")
}

#[test]
fn check_exits_zero_when_completed_without_diagnostics() {
    let output = run_cjkfmt(&["--max-width", "3", "check"], "abc\n", None);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn check_exits_one_when_completed_with_diagnostics_without_changing_output() {
    let output = run_cjkfmt(&["--max-width", "2", "check"], "abc\n", None);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "<stdin>:1:3: W001 Line length exceeds 2 characters\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn check_exits_one_when_a_named_file_has_diagnostics() {
    let output = run_cjkfmt_with_input(
        &["--max-width", "2", "check", "document.md"],
        b"",
        None,
        &[("document.md", "abc\n")],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "document.md:1:3: W001 Line length exceeds 2 characters\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn format_exits_one_when_an_input_file_cannot_be_read() {
    let output = run_cjkfmt(&["format", "missing.md"], "", None);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
}

#[test]
fn format_stops_python_stdout_batches_at_syntax_and_input_errors() {
    let failing_inputs: [(&str, &[u8], &str); 3] = [
        (
            "syntax",
            "# 漢A\nif :\n".as_bytes(),
            "Python syntax parse failed",
        ),
        ("invalid UTF-8", &[b'#', b' ', 0xff], "valid UTF-8"),
        (
            "incompatible encoding",
            "# coding: latin-1\n# 漢A\n".as_bytes(),
            "not supported; use UTF-8",
        ),
    ];

    for (case, failing_input, error_hint) in failing_inputs {
        let directory = tempdir().expect("temporary directory should be created");
        std::fs::write(directory.path().join("valid.py"), "# 漢A\n").unwrap();
        std::fs::write(directory.path().join("invalid.py"), failing_input).unwrap();
        std::fs::write(directory.path().join("later.py"), "# 漢A\n").unwrap();

        let output = run_cjkfmt_in_directory(
            &[
                "--spacing-alphabets",
                "require",
                "format",
                "valid.py",
                "invalid.py",
                "later.py",
            ],
            directory.path(),
        );

        assert_eq!(output.status.code(), Some(1), "{case}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "# 漢 A\n",
            "{case}"
        );
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains(error_hint),
            "{case}"
        );
        assert_eq!(
            std::fs::read(directory.path().join("invalid.py")).unwrap(),
            failing_input,
            "failed input should remain unchanged: {case}"
        );
        assert_eq!(
            std::fs::read_to_string(directory.path().join("later.py")).unwrap(),
            "# 漢A\n",
            "later input should not be processed: {case}"
        );
    }
}

#[test]
fn format_write_batches_keep_completed_files_and_skip_failed_and_later_python_inputs() {
    let failing_inputs: [(&str, &[u8], &str); 3] = [
        (
            "syntax",
            "# 漢A\nif :\n".as_bytes(),
            "Python syntax parse failed",
        ),
        ("invalid UTF-8", &[b'#', b' ', 0xff], "valid UTF-8"),
        (
            "incompatible encoding",
            "# coding: latin-1\n# 漢A\n".as_bytes(),
            "not supported; use UTF-8",
        ),
    ];

    for (case, failing_input, error_hint) in failing_inputs {
        let directory = tempdir().expect("temporary directory should be created");
        std::fs::write(directory.path().join("valid.py"), "# 漢A\n").unwrap();
        std::fs::write(directory.path().join("invalid.py"), failing_input).unwrap();
        std::fs::write(directory.path().join("later.py"), "# 漢A\n").unwrap();

        let output = run_cjkfmt_in_directory(
            &[
                "--spacing-alphabets",
                "require",
                "format",
                "--write",
                "valid.py",
                "invalid.py",
                "later.py",
            ],
            directory.path(),
        );

        assert_eq!(output.status.code(), Some(1), "{case}");
        assert!(output.stdout.is_empty(), "{case}");
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains(error_hint),
            "{case}"
        );
        assert_eq!(
            std::fs::read_to_string(directory.path().join("valid.py")).unwrap(),
            "# 漢 A\n",
            "completed file should remain formatted: {case}"
        );
        assert_eq!(
            std::fs::read(directory.path().join("invalid.py")).unwrap(),
            failing_input,
            "failed input should remain unchanged: {case}"
        );
        assert_eq!(
            std::fs::read_to_string(directory.path().join("later.py")).unwrap(),
            "# 漢A\n",
            "later input should not be processed: {case}"
        );
    }
}

#[test]
fn format_batches_keep_completed_output_and_stop_at_an_unreadable_python_file() {
    for write in [false, true] {
        let directory = tempdir().expect("temporary directory should be created");
        std::fs::write(directory.path().join("valid.py"), "# 漢A\n").unwrap();
        std::fs::write(directory.path().join("later.py"), "# 漢A\n").unwrap();

        let mut args = vec!["--spacing-alphabets", "require", "format"];
        if write {
            args.push("--write");
        }
        args.extend(["valid.py", "missing.py", "later.py"]);
        let output = run_cjkfmt_in_directory(&args, directory.path());

        assert_eq!(output.status.code(), Some(1), "write={write}");
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with("Error: "),
            "write={write}"
        );
        if write {
            assert!(output.stdout.is_empty());
            assert_eq!(
                std::fs::read_to_string(directory.path().join("valid.py")).unwrap(),
                "# 漢 A\n"
            );
        } else {
            assert_eq!(String::from_utf8(output.stdout).unwrap(), "# 漢 A\n");
            assert_eq!(
                std::fs::read_to_string(directory.path().join("valid.py")).unwrap(),
                "# 漢A\n"
            );
        }
        assert_eq!(
            std::fs::read_to_string(directory.path().join("later.py")).unwrap(),
            "# 漢A\n",
            "later input should not be processed: write={write}"
        );
    }
}

#[test]
fn check_exits_two_when_stdin_contains_invalid_utf8() {
    let output = run_cjkfmt_with_input(&["check"], &[0xff], None, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
}

#[test]
fn check_exits_two_when_an_input_file_cannot_be_read() {
    let output = run_cjkfmt(&["check", "missing.md"], "", None);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
}

#[test]
fn check_exits_two_when_configuration_is_invalid() {
    let output = run_cjkfmt(&["check"], "", Some(r#"{"max_width":"invalid"}"#));

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("failed to parse configuration")
    );
}

#[test]
fn format_and_check_infer_python_extensions_and_only_process_comments() {
    let source = "\"\"\"漢A\"\"\"\n# 漢A\nvalue = \"漢A\"  # 漢A\n";
    for filename in [
        "document.py",
        "document.Py",
        "document.pyi",
        "document.PyI",
        "document.pyw",
        "document.pYw",
    ] {
        let formatted = run_cjkfmt_with_input(
            &["--spacing-alphabets", "require", "format", filename],
            b"",
            None,
            &[(filename, source)],
        );
        assert_eq!(formatted.status.code(), Some(0), "{filename}");
        assert_eq!(
            String::from_utf8(formatted.stdout).unwrap(),
            "\"\"\"漢A\"\"\"\n# 漢 A\nvalue = \"漢A\"  # 漢 A\n",
            "{filename}"
        );
        assert!(formatted.stderr.is_empty(), "{filename}");

        let checked = run_cjkfmt_with_input(
            &["--spacing-alphabets", "require", "check", filename],
            b"",
            None,
            &[(filename, source)],
        );
        assert_eq!(checked.status.code(), Some(1), "{filename}");
        assert_eq!(
            String::from_utf8(checked.stdout).unwrap(),
            format!(
                "{filename}:2:4: W002 Possible spacing position found\n\
                 {filename}:3:18: W002 Possible spacing position found\n"
            ),
            "{filename}"
        );
        assert!(checked.stderr.is_empty(), "{filename}");
    }
}

#[test]
fn format_accepts_explicit_python_for_stdin_without_a_filename() {
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        "# 漢A\nvalue = \"漢A\"\n",
        None,
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "# 漢 A\nvalue = \"漢A\"\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn check_reports_python_comment_spacing_at_original_source_positions() {
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "check",
            "--language",
            "python",
        ],
        "# 漢A\n",
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "<stdin>:1:4: W002 Possible spacing position found\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn check_infers_python_for_py_and_explicit_python_overrides_other_extensions() {
    let inferred = run_cjkfmt_with_input(
        &["--spacing-alphabets", "require", "check", "document.PY"],
        b"",
        None,
        &[("document.PY", "# 漢A\n")],
    );
    assert_eq!(inferred.status.code(), Some(1));
    assert!(String::from_utf8(inferred.stdout).unwrap().contains("W002"));

    let explicit = run_cjkfmt_with_input(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
            "document.txt",
        ],
        b"",
        None,
        &[("document.txt", "# 漢A\n")],
    );
    assert_eq!(explicit.status.code(), Some(0));
    assert_eq!(String::from_utf8(explicit.stdout).unwrap(), "# 漢 A\n");

    let override_python = run_cjkfmt_with_input(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "json",
            "document.PyI",
        ],
        b"",
        None,
        &[("document.PyI", "# 漢A\n")],
    );
    assert_eq!(override_python.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(override_python.stdout).unwrap(),
        "# 漢A\n"
    );
}

#[test]
fn python_formatting_is_idempotent_and_unknown_extensions_are_not_inferred() {
    let formatted = "# 漢 A\nvalue = \"漢A\"  # 漢 A\n";
    let second_pass = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        formatted,
        None,
    );
    assert_eq!(second_pass.status.code(), Some(0));
    assert_eq!(String::from_utf8(second_pass.stdout).unwrap(), formatted);

    let unknown = run_cjkfmt_with_input(
        &["--spacing-alphabets", "require", "format", "document.pyx"],
        b"",
        None,
        &[("document.pyx", "# 漢A\n")],
    );
    assert_eq!(unknown.status.code(), Some(0));
    assert_eq!(String::from_utf8(unknown.stdout).unwrap(), "# 漢A\n");
}

#[test]
fn python_rejects_a_bom_conflicting_with_the_source_encoding_declaration() {
    let output = run_cjkfmt(
        &["format", "--language", "python"],
        "\u{feff}# coding: latin-1\n# 漢A\n",
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("BOM conflicts with the Python encoding declaration")
    );
}

#[test]
fn check_python_spacing_positions_ignore_the_utf8_bom_signature() {
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "check",
            "--language",
            "python",
        ],
        "\u{feff}# 漢A\n",
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "<stdin>:1:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn check_keeps_python_line_length_diagnostics_when_syntax_is_invalid() {
    let output = run_cjkfmt(
        &["--max-width", "5", "check", "--language", "python"],
        "# 123456\nif :\n",
        None,
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "<stdin>:1:6: W001 Line length exceeds 5 characters\n"
    );
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Python syntax parse failed")
    );
}

#[test]
fn check_preserves_prior_and_independent_diagnostics_before_stopping_at_python_syntax_errors() {
    let output = run_cjkfmt_with_input(
        &[
            "--spacing-alphabets",
            "require",
            "--max-width",
            "5",
            "check",
            "valid.py",
            "invalid.py",
            "later.py",
        ],
        b"",
        None,
        &[
            ("valid.py", "# 漢A\n"),
            ("invalid.py", "# 漢A 123456\nif :\n"),
            ("later.py", "# 漢A\n"),
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "valid.py:1:4: W002 Possible spacing position found\n\
         invalid.py:1:5: W001 Line length exceeds 5 characters\n"
    );
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Python syntax parse failed")
    );
}

#[test]
fn check_keeps_prior_diagnostics_without_claiming_w001_for_undecodable_batch_input() {
    let directory = tempdir().expect("temporary directory should be created");
    std::fs::write(directory.path().join("valid.py"), "# 漢A\n").unwrap();
    std::fs::write(directory.path().join("invalid.py"), [b'#', b' ', 0xff]).unwrap();
    std::fs::write(directory.path().join("later.py"), "# 漢A\n").unwrap();

    let output = run_cjkfmt_in_directory(
        &[
            "--spacing-alphabets",
            "require",
            "check",
            "valid.py",
            "invalid.py",
            "later.py",
        ],
        directory.path(),
    );

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "valid.py:1:4: W002 Possible spacing position found\n"
    );
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
}

#[test]
fn python_spacing_rules_cover_standalone_and_trailing_comments_without_wrapping() {
    for (rule, source, expected) in [
        (
            "require",
            "# 漢A\nvalue = 1  # 漢2\n",
            "# 漢 A\nvalue = 1  # 漢 2\n",
        ),
        (
            "prohibit",
            "# 漢 A\nvalue = 1  # 漢 2\n",
            "# 漢A\nvalue = 1  # 漢2\n",
        ),
        (
            "ignore",
            "# 漢A\nvalue = 1  # 漢2\n",
            "# 漢A\nvalue = 1  # 漢2\n",
        ),
    ] {
        for wrapping in ["preserve", "split"] {
            let output = run_cjkfmt(
                &[
                    "--spacing-alphabets",
                    rule,
                    "--spacing-digits",
                    rule,
                    "--line-wrapping",
                    wrapping,
                    "--max-width",
                    "3",
                    "format",
                    "--language",
                    "python",
                ],
                source,
                None,
            );
            assert_eq!(
                output.status.code(),
                Some(0),
                "rule={rule}, wrapping={wrapping}"
            );
            assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
fn python_alphabet_and_digit_spacing_rules_remain_independent() {
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "ignore",
            "--spacing-digits",
            "require",
            "format",
            "--language",
            "python",
        ],
        "# 漢A 漢1\n",
        None,
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "# 漢A 漢 1\n");
}

#[test]
fn python_comment_group_formats_prose_around_protected_markup() {
    let source = "# 漢A\n# ```python\n# print(\"漢A\")\n# ```\n# 漢A\n";
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
        None,
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "# 漢 A\n# ```python\n# print(\"漢A\")\n# ```\n# 漢 A\n"
    );
}

#[test]
fn python_accepts_common_utf8_encoding_aliases() {
    for encoding in ["UTF_8", "utf8", "u8", "cp65001", "utf-8-sig"] {
        let source = format!("# coding: {encoding}\n# 漢A\n");
        let output = run_cjkfmt(
            &[
                "--spacing-alphabets",
                "require",
                "format",
                "--language",
                "python",
            ],
            &source,
            None,
        );

        assert_eq!(output.status.code(), Some(0), "encoding={encoding}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("# coding: {encoding}\n# 漢 A\n"),
            "encoding={encoding}"
        );
    }
}

#[test]
fn python_encoding_detection_ignores_words_after_coding() {
    let source = "# coding guidelines: examples\n# 漢A\n";
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
        None,
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "# coding guidelines: examples\n# 漢 A\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn python_ignores_encoding_cookies_after_code_on_the_first_line() {
    let source = "value = 1\n# coding: latin-1\n# 漢A\n";
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
        None,
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "value = 1\n# coding: latin-1\n# 漢 A\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn python_trailing_comments_with_protected_markup_remain_unchanged() {
    let source = "x = 1  # `漢A`\nvalue = 2  # https://example.com/漢A\n# 漢A\n";
    let expected = "x = 1  # `漢A`\nvalue = 2  # https://example.com/漢A\n# 漢 A\n";
    let format = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
        None,
    );
    assert_eq!(format.status.code(), Some(0));
    assert_eq!(String::from_utf8(format.stdout).unwrap(), expected);

    let check = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "check",
            "--language",
            "python",
        ],
        source,
        None,
    );
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(check.stdout).unwrap(),
        "<stdin>:3:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn python_bare_cr_lines_keep_code_and_strings_outside_comment_edits() {
    let source = "# 漢A\rvalue = \"漢B\"\r# 漢C\n";
    let expected = "# 漢 A\rvalue = \"漢B\"\r# 漢 C\n";
    let format = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
        None,
    );
    assert_eq!(format.status.code(), Some(0));
    assert_eq!(String::from_utf8(format.stdout).unwrap(), expected);

    let check = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "check",
            "--language",
            "python",
        ],
        source,
        None,
    );
    assert_eq!(check.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(check.stdout).unwrap(),
        "<stdin>:1:4: W002 Possible spacing position found\n\
         <stdin>:3:4: W002 Possible spacing position found\n"
    );
}

#[test]
fn python_format_rejects_syntax_errors_without_emitting_formatted_source() {
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        "# 漢A\nif :\n",
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Python syntax parse failed")
    );
}

#[test]
fn python_format_preserves_bom_and_mixed_physical_line_endings() {
    let source = "\u{feff}# coding: UTF_8\r\n# 漢A\r\nvalue = 1  # 漢A\n";
    let expected = "\u{feff}# coding: UTF_8\r\n# 漢 A\r\nvalue = 1  # 漢 A\n";
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
        None,
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn python_rejects_a_non_utf8_encoding_cookie_after_a_shebang() {
    let output = run_cjkfmt(
        &["format", "--language", "python"],
        "#!/usr/bin/env python\n# coding: latin-1\n# 漢A\n",
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("Python source encoding \"latin-1\" is not supported")
    );
}

#[test]
fn python_format_rejects_non_utf8_encoding_declarations() {
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        "# coding: latin-1\n# 漢A\n",
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("not supported; use UTF-8")
    );
}

#[test]
fn python_write_formats_valid_files_and_leaves_syntax_failures_untouched() {
    let directory = tempdir().expect("temporary directory should be created");
    let valid_path = directory.path().join("valid.pYw");
    let invalid_path = directory.path().join("invalid.PyI");
    std::fs::write(&valid_path, "# 漢A\r\n").unwrap();
    std::fs::write(&invalid_path, "# 漢A\nif :\n").unwrap();

    let valid = Command::new(env!("CARGO_BIN_EXE_cjkfmt"))
        .args([
            "--color",
            "never",
            "--spacing-alphabets",
            "require",
            "format",
            "--write",
        ])
        .arg(&valid_path)
        .current_dir(directory.path())
        .env_clear()
        .env("HOME", directory.path())
        .env("XDG_CONFIG_HOME", directory.path())
        .output()
        .expect("format should run");
    assert_eq!(valid.status.code(), Some(0));
    assert!(valid.stdout.is_empty());
    assert_eq!(std::fs::read(&valid_path).unwrap(), "# 漢 A\r\n".as_bytes());

    let original_invalid = std::fs::read(&invalid_path).unwrap();
    let invalid = Command::new(env!("CARGO_BIN_EXE_cjkfmt"))
        .args([
            "--color",
            "never",
            "--spacing-alphabets",
            "require",
            "format",
            "--write",
        ])
        .arg(&invalid_path)
        .current_dir(directory.path())
        .env_clear()
        .env("HOME", directory.path())
        .env("XDG_CONFIG_HOME", directory.path())
        .output()
        .expect("format should run");
    assert_eq!(invalid.status.code(), Some(1));
    assert!(invalid.stdout.is_empty());
    assert!(
        String::from_utf8(invalid.stderr)
            .unwrap()
            .contains("Python syntax parse failed")
    );
    assert_eq!(std::fs::read(&invalid_path).unwrap(), original_invalid);
}

#[test]
fn python_write_leaves_encoding_rejected_files_unchanged() {
    let directory = tempdir().expect("temporary directory should be created");
    let path = directory.path().join("encoded.py");
    let original = "# coding: latin-1\n# 漢A\n";
    std::fs::write(&path, original).unwrap();

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
        .expect("format should run");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("not supported; use UTF-8")
    );
    assert_eq!(std::fs::read(&path).unwrap(), original.as_bytes());
}

#[test]
fn python_cli_rejects_invalid_utf8_without_emitting_formatted_output_or_diagnostics() {
    let format = run_cjkfmt_with_input(&["format", "--language", "python"], &[0xff], None, &[]);
    assert_eq!(format.status.code(), Some(1));
    assert!(format.stdout.is_empty());
    assert!(
        String::from_utf8(format.stderr)
            .unwrap()
            .starts_with("Error: ")
    );

    let check = run_cjkfmt_with_input(&["check", "--language", "python"], &[0xff], None, &[]);
    assert_eq!(check.status.code(), Some(2));
    assert!(check.stdout.is_empty());
    assert!(
        String::from_utf8(check.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
}

#[test]
fn python_comment_regions_outside_protected_groups_remain_formatable() {
    let source = "# 漢A\n\n# ```python\n# 漢A\n\n# 漢A\n";
    let expected = "# 漢 A\n\n# ```python\n# 漢A\n\n# 漢 A\n";
    let output = run_cjkfmt(
        &[
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "python",
        ],
        source,
        None,
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn invalid_command_line_arguments_exit_two() {
    let output = run_cjkfmt(&["--unknown-option", "check"], "", None);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unexpected argument")
    );
}
