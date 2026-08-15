use std::{
    fs,
    io::{Read, stdin},
    path::Path,
};

use crate::{config::Config, format::format_one_file};

pub fn format_command<W: std::io::Write, P: AsRef<Path>>(
    stdout: &mut W,
    config: &Config,
    filenames: &[P],
) -> anyhow::Result<()> {
    let mut stdin = stdin();
    format_command_with_reader(stdout, config, filenames, &mut stdin)
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
}

fn format_command_with_reader<W, P, R>(
    stdout: &mut W,
    config: &Config,
    filenames: &[P],
    stdin: &mut R,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: AsRef<Path>,
    R: Read,
{
    // Read content of the specified files or standard input. Without a
    // filename, the input type is unknown, so Markdown spacing is opt-in
    // rather than assumed.
    if filenames.is_empty() {
        let mut content = String::with_capacity(1024);
        stdin.read_to_string(&mut content)?;
        format_one_file(stdout, config, false, &content)?;
    } else {
        for filename in filenames {
            let filename = filename.as_ref();
            let apply_markdown_spacing = is_markdown_path(filename);
            let content = fs::read_to_string(filename)?;
            format_one_file(stdout, config, apply_markdown_spacing, &content)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use super::*;
    use crate::config::{Config, SpacingRule};

    fn config() -> Config {
        let mut config = Config {
            max_width: 200,
            ..Config::default()
        };
        config.spacing.alphabets = SpacingRule::Require;
        config
    }

    #[test]
    fn format_command_applies_spacing_only_to_identified_markdown_files() {
        let directory = tempdir().unwrap();
        let cases = [
            ("document.md", "漢 A\n"),
            ("document.markdown", "漢 A\n"),
            ("document.MD", "漢 A\n"),
            ("document.MarkDown", "漢 A\n"),
            ("document.txt", "漢A\n"),
            ("document.rs", "漢A\n"),
            ("document.JSON", "漢A\n"),
            ("document", "漢A\n"),
            ("document.md.txt", "漢A\n"),
        ];
        let mut paths = Vec::with_capacity(cases.len());
        for (filename, _) in cases {
            let path = directory.path().join(filename);
            fs::write(&path, "漢A\n").unwrap();
            paths.push(path);
        }

        let mut output = Vec::new();
        format_command(&mut output, &config(), &paths).unwrap();

        let expected: String = cases.iter().map(|(_, output)| *output).collect();
        assert_eq!(String::from_utf8(output).unwrap(), expected);
        // `TempDir` removes the directory during unwinding as well as on
        // success, so assertion failures do not leave test files behind.
    }

    #[test]
    fn format_command_does_not_assume_stdin_is_markdown() {
        let mut input = "漢A\n".as_bytes();
        let mut output = Vec::new();

        format_command_with_reader(&mut output, &config(), &[] as &[PathBuf], &mut input).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "漢A\n");
    }
}
