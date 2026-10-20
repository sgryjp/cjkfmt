use std::{io::Write, process::Command};

use tempfile::tempdir;

fn run_formatter(
    working_directory: &std::path::Path,
    home_directory: &std::path::Path,
    line_wrapping_env: Option<&str>,
    line_wrapping_cli: Option<&str>,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cjkfmt"));
    command
        .current_dir(working_directory)
        .env_clear()
        .env("HOME", home_directory)
        .args(["--max-width", "4"]);
    if let Some(value) = line_wrapping_env {
        command.env("CJKFMT_LINE_WRAPPING", value);
    }
    if let Some(value) = line_wrapping_cli {
        command.args(["--line-wrapping", value]);
    }
    command.arg("format").args(["--language", "markdown"]);
    let mut child = command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("cjkfmt subprocess should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(b"aaaa bbbb")
        .expect("input should be written");
    child.wait_with_output().expect("cjkfmt should exit")
}

#[test]
fn line_wrapping_configuration_obeys_file_environment_and_cli_precedence() {
    let directory = tempdir().unwrap();
    let home = tempdir().unwrap();

    std::fs::write(
        directory.path().join(".cjkfmt.json"),
        r#"{ "max_width": 4, "line_wrapping": "preserve" }"#,
    )
    .unwrap();
    let output = run_formatter(directory.path(), home.path(), None, None);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"aaaa bbbb");

    let output = run_formatter(directory.path(), home.path(), Some("split"), None);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"aaaa\nbbbb");

    std::fs::write(
        directory.path().join(".cjkfmt.json"),
        r#"{ "max_width": 4, "line_wrapping": "split" }"#,
    )
    .unwrap();
    let output = run_formatter(directory.path(), home.path(), Some("preserve"), None);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"aaaa bbbb");

    let output = run_formatter(
        directory.path(),
        home.path(),
        Some("preserve"),
        Some("split"),
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"aaaa\nbbbb");
}

#[test]
fn preserve_mode_keeps_existing_lines_when_the_cli_writes_a_file() {
    let directory = tempdir().unwrap();
    let home = tempdir().unwrap();
    let input_path = directory.path().join("document.md");
    std::fs::write(&input_path, "漢A one two\r\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cjkfmt"))
        .current_dir(directory.path())
        .env_clear()
        .env("HOME", home.path())
        .args([
            "--max-width",
            "4",
            "--line-wrapping",
            "preserve",
            "--spacing-alphabets",
            "require",
            "format",
            "--language",
            "markdown",
            "--write",
        ])
        .arg(&input_path)
        .output()
        .expect("cjkfmt subprocess should start");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        std::fs::read(&input_path).unwrap(),
        "漢 A one two\r\n".as_bytes()
    );
}

#[test]
fn invalid_line_wrapping_environment_values_fail_configuration_loading() {
    let directory = tempdir().unwrap();
    let home = tempdir().unwrap();
    let output = run_formatter(directory.path(), home.path(), Some("wrap"), None);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("LINE_WRAPPING"),
        "unexpected configuration error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
