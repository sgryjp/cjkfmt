use std::{
    io::{Read, stdin},
    path::Path,
};

pub fn debug_cst_command<W, P>(stdout: &mut W, filenames: &[P]) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
{
    let mut stdin = stdin();
    debug_cst_command_with_reader(stdout, filenames, &mut stdin)
}

fn debug_cst_command_with_reader<W, P, R>(
    _stdout: &mut W,
    _filenames: &[P],
    _stdin: &mut R,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
    R: Read,
{
    todo!()
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
    fn debug_cst_command_uses_json_grammar_for_json_files() {
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
