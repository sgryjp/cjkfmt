use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

fn run_cjkfmt(args: &[&str], input: &str, config: Option<&str>) -> Output {
    run_cjkfmt_with_input(args, input.as_bytes(), config, &[])
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
fn invalid_command_line_arguments_exit_two() {
    let output = run_cjkfmt(&["--unknown-option", "check"], "", None);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unexpected argument")
    );
}
