use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{config::Config, core::diagnostic::Diagnostic, document::Document, language::Language};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CheckTestCaseDefinition {
    pub(super) input_file: PathBuf,
    pub(super) language: Option<Language>,
    pub(super) config: Config,
    pub(super) diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub(super) struct LoadedCheckTestCase {
    pub(super) config: Config,
    pub(super) document: Document,
    pub(super) diagnostics: Vec<Diagnostic>,
}

pub(super) fn load_check_test_case(
    case_json_path: &Path,
    repository_root: &Path,
) -> Result<LoadedCheckTestCase> {
    let repository_root = repository_root.canonicalize().with_context(|| {
        format!(
            "failed to resolve repository root {}",
            repository_root.display()
        )
    })?;
    let case_json_path = resolve_path(case_json_path, &repository_root);
    let case_directory = case_json_path
        .parent()
        .context("case definition has no parent directory")?;
    let definition = read_check_test_case_definition(&case_json_path, &repository_root)?;

    if definition.input_file.is_absolute() {
        anyhow::bail!(
            "input_file in {} must be relative to its case directory",
            case_json_path.display()
        );
    }

    let canonical_case_directory = case_directory.canonicalize().with_context(|| {
        format!(
            "failed to resolve case directory {}",
            case_directory.display()
        )
    })?;
    let input_path = canonical_case_directory.join(&definition.input_file);
    let canonical_input_path = input_path.canonicalize().with_context(|| {
        format!(
            "failed to resolve input_file {:?} for case {}",
            definition.input_file,
            case_json_path.display()
        )
    })?;
    if !canonical_input_path.starts_with(&canonical_case_directory) {
        anyhow::bail!(
            "input_file {:?} in {} resolves outside its case directory",
            definition.input_file,
            case_json_path.display()
        );
    }
    if !canonical_input_path.is_file() {
        anyhow::bail!(
            "input_file {:?} in {} is not a regular file",
            definition.input_file,
            case_json_path.display()
        );
    }

    let input = fs::read_to_string(&canonical_input_path).with_context(|| {
        format!(
            "failed to read UTF-8 input_file {:?} for case {}",
            definition.input_file,
            case_json_path.display()
        )
    })?;
    let language = Language::explicit_or_inferred_path(definition.language, &definition.input_file)
        .with_context(|| {
            format!(
                "cannot determine language for input_file {:?} in {}",
                definition.input_file,
                case_json_path.display()
            )
        })?;

    let relative_case_directory =
        case_directory
            .strip_prefix(&repository_root)
            .with_context(|| {
                format!(
                    "case {} is outside repository root {}",
                    case_json_path.display(),
                    repository_root.display()
                )
            })?;
    let filename = relative_case_directory
        .join(&definition.input_file)
        .to_string_lossy()
        .replace('\\', "/");
    let diagnostics = definition
        .diagnostics
        .into_iter()
        .map(|mut diagnostic| {
            diagnostic.filename = Some(filename.clone());
            diagnostic
        })
        .collect();

    Ok(LoadedCheckTestCase {
        config: definition.config,
        document: Document::new(input, Some(language), Some(filename)),
        diagnostics,
    })
}

fn resolve_path(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

pub(super) fn read_check_test_case_definition(
    case_json_path: &Path,
    repository_root: &Path,
) -> Result<CheckTestCaseDefinition> {
    let case_json_path = if case_json_path.is_absolute() {
        case_json_path.to_path_buf()
    } else {
        repository_root.join(case_json_path)
    };
    let content = fs::read_to_string(&case_json_path).with_context(|| {
        format!(
            "failed to read case definition {}",
            case_json_path.display()
        )
    })?;
    serde_json::from_str(&content).with_context(|| {
        format!(
            "failed to parse case definition {}",
            case_json_path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn loaded_input_preserves_bytes_and_uses_the_declared_relative_filename() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/byte-preservation");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        let input_bytes = "# 漢 1\r\n trailing \t".as_bytes();
        fs::write(case_directory.join("input.md"), input_bytes)
            .expect("input document should be written");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.md","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let loaded = load_check_test_case(
            Path::new("test_cases/check/byte-preservation/case.json"),
            repository_root.path(),
        )
        .expect("the case should load");

        assert_eq!(loaded.document.content.as_bytes(), input_bytes);
        assert_eq!(
            loaded.document.filename.as_deref(),
            Some("test_cases/check/byte-preservation/input.md")
        );
        assert_eq!(loaded.document.language, Some(Language::Markdown));
    }

    #[cfg(unix)]
    #[test]
    fn internal_input_symlinks_keep_the_declared_filename() {
        use std::os::unix::fs::symlink;

        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/internal-link");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(case_directory.join("source.md"), "content")
            .expect("symlink target should be written");
        symlink("source.md", case_directory.join("input.md"))
            .expect("input symlink should be created");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.md","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let loaded = load_check_test_case(
            Path::new("test_cases/check/internal-link/case.json"),
            repository_root.path(),
        )
        .expect("an in-case symlink should load");

        assert_eq!(loaded.document.content, "content");
        assert_eq!(
            loaded.document.filename.as_deref(),
            Some("test_cases/check/internal-link/input.md")
        );
    }

    #[cfg(unix)]
    #[test]
    fn input_files_reject_symlinks_that_resolve_outside_the_case() {
        use std::os::unix::fs::symlink;

        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/external-link");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        let outside_input = repository_root.path().join("outside.md");
        fs::write(&outside_input, "outside").expect("outside input should be written");
        symlink(&outside_input, case_directory.join("input.md"))
            .expect("input symlink should be created");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.md","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let error = load_check_test_case(
            Path::new("test_cases/check/external-link/case.json"),
            repository_root.path(),
        )
        .expect_err("a symlink to an input outside its case should fail");

        assert!(
            format!("{error:#}").contains("outside its case directory"),
            "error should identify the case boundary violation: {error:#}"
        );
    }

    #[test]
    fn input_files_with_invalid_utf8_report_the_input_path() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root.path().join("test_cases/check/invalid-utf8");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(case_directory.join("input.md"), [0xff])
            .expect("invalid UTF-8 input should be written");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.md","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let error = load_check_test_case(
            Path::new("test_cases/check/invalid-utf8/case.json"),
            repository_root.path(),
        )
        .expect_err("non-UTF-8 input should fail");
        let error = format!("{error:#}");

        assert!(error.contains("input.md"), "{error}");
        assert!(error.contains("UTF-8"), "{error}");
    }

    #[test]
    fn missing_input_files_report_the_case_and_input_name() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root.path().join("test_cases/check/missing-file");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"absent.md","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let error = load_check_test_case(
            Path::new("test_cases/check/missing-file/case.json"),
            repository_root.path(),
        )
        .expect_err("a missing input should fail");
        let error = format!("{error:#}");

        assert!(error.contains("missing-file/case.json"), "{error}");
        assert!(error.contains("absent.md"), "{error}");
    }

    #[test]
    fn input_files_reject_parent_directory_escapes() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/escaped-input");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(
            repository_root.path().join("test_cases/check/outside.md"),
            "outside",
        )
        .expect("outside input should be written");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"../outside.md","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let error = load_check_test_case(
            Path::new("test_cases/check/escaped-input/case.json"),
            repository_root.path(),
        )
        .expect_err("an input outside its case should fail");

        assert!(
            format!("{error:#}").contains("outside its case directory"),
            "error should identify the case boundary violation: {error:#}"
        );
    }

    #[test]
    fn input_files_reject_absolute_paths() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/absolute-input");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(case_directory.join("input.md"), "input")
            .expect("input document should be written");
        let absolute_input = case_directory.join("input.md");
        let definition = serde_json::json!({
            "input_file": absolute_input,
            "config": {},
            "diagnostics": []
        });
        fs::write(case_directory.join("case.json"), definition.to_string())
            .expect("case definition should be written");

        let error = load_check_test_case(
            Path::new("test_cases/check/absolute-input/case.json"),
            repository_root.path(),
        )
        .expect_err("an absolute input path should fail");

        assert!(
            format!("{error:#}").contains("must be relative"),
            "error should explain the relative path requirement: {error:#}"
        );
    }

    #[test]
    fn case_definitions_reject_unsupported_language_values() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/invalid-language");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.md","language":"plaintext","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let error = read_check_test_case_definition(
            Path::new("test_cases/check/invalid-language/case.json"),
            repository_root.path(),
        )
        .expect_err("an unsupported language should fail");

        assert!(
            format!("{error:#}").contains("plaintext"),
            "error should identify the unsupported language: {error:#}"
        );
    }

    #[test]
    fn json_input_files_select_the_json_language_by_extension() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root.path().join("test_cases/check/json-input");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(case_directory.join("input.json"), r#"{"key": "value"}"#)
            .expect("input document should be written");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.json","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let loaded = load_check_test_case(
            Path::new("test_cases/check/json-input/case.json"),
            repository_root.path(),
        )
        .expect("the case should load");

        assert_eq!(loaded.document.language, Some(Language::Json));
    }

    #[test]
    fn input_files_without_a_supported_extension_require_an_explicit_language() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/unknown-language");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(case_directory.join("input.txt"), "input")
            .expect("input document should be written");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.txt","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let error = load_check_test_case(
            Path::new("test_cases/check/unknown-language/case.json"),
            repository_root.path(),
        )
        .expect_err("an unknown extension should not default to Markdown");

        assert!(
            format!("{error:#}").contains("cannot determine language"),
            "error should explain why language selection failed: {error:#}"
        );
    }

    #[test]
    fn explicit_language_overrides_the_input_file_extension() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/language-override");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(case_directory.join("input.json"), "not parsed as JSON")
            .expect("input document should be written");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.json","language":"markdown","config":{},"diagnostics":[]}"#,
        )
        .expect("case definition should be written");

        let loaded = load_check_test_case(
            Path::new("test_cases/check/language-override/case.json"),
            repository_root.path(),
        )
        .expect("the case should load");

        assert_eq!(loaded.document.language, Some(Language::Markdown));
    }

    #[test]
    fn check_case_definitions_require_config_and_diagnostics() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/missing-required");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        for (definition, missing_field) in [
            (r#"{"input_file":"input.md","diagnostics":[]}"#, "config"),
            (r#"{"input_file":"input.md","config":{}}"#, "diagnostics"),
        ] {
            fs::write(case_directory.join("case.json"), definition)
                .expect("case definition should be written");

            let error = read_check_test_case_definition(
                Path::new("test_cases/check/missing-required/case.json"),
                repository_root.path(),
            )
            .expect_err("a definition missing a required field should fail");

            assert!(
                format!("{error:#}").contains(missing_field),
                "error should identify the missing {missing_field} field: {error:#}"
            );
        }
    }

    #[test]
    fn check_case_definitions_reject_unknown_top_level_fields() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/unknown-field");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(
            case_directory.join("case.json"),
            r#"{"input_file":"input.md","config":{},"diagnostics":[],"unexpected":true}"#,
        )
        .expect("case definition should be written");

        let error = read_check_test_case_definition(
            Path::new("test_cases/check/unknown-field/case.json"),
            repository_root.path(),
        )
        .expect_err("a case definition with an unknown field should fail");

        assert!(
            format!("{error:#}").contains("unexpected"),
            "error should identify the unknown field: {error:#}"
        );
    }

    #[test]
    fn check_case_definitions_require_an_input_file() {
        let repository_root = tempfile::tempdir().expect("temporary repository should be created");
        let case_directory = repository_root
            .path()
            .join("test_cases/check/missing-input");
        fs::create_dir_all(&case_directory).expect("case directory should be created");
        fs::write(
            case_directory.join("case.json"),
            r#"{"config": {}, "diagnostics": []}"#,
        )
        .expect("case definition should be written");

        let error = read_check_test_case_definition(
            Path::new("test_cases/check/missing-input/case.json"),
            repository_root.path(),
        )
        .expect_err("a case definition without input_file should fail");

        assert!(
            format!("{error:#}").contains("input_file"),
            "error should identify the missing input_file field: {error:#}"
        );
    }
}
