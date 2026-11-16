use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

use tempfile::tempdir;

pub fn run_cjkfmt(args: &[&str], input: &str) -> Output {
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
