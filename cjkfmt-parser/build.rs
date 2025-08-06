use std::{env, fs, path::PathBuf, process::Command};

use cc::Build;
use url::Url;

struct GrammarSpec {
    library_name: &'static str, // must be unique
    repo_url: Url,
    rev: &'static str,
    subdir: &'static str,
    source: Vec<&'static str>,
}

// Helper function to execute shell commands with error handling
fn exec(command: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new(command).args(args).status().map_err(|e| {
        format!(
            "Failed to execute command [[ {command} {} ]]. {e}",
            args.join(" ")
        )
    })?;
    if !status.success() {
        return Err(format!(
            "Command finished with exit status {}: {command} {}",
            status.code().unwrap_or(-1),
            args.join(" ")
        )
        .into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define the tree-sitter grammars to be compiled
    let grammar_specs = vec![
        GrammarSpec {
            library_name: "markdown",
            repo_url: Url::parse("https://github.com/tree-sitter-grammars/tree-sitter-markdown")
                .unwrap(),
            rev: "7462bb66ac7e90312082269007fac2772fe5efd1",
            subdir: "tree-sitter-markdown",
            source: vec!["src/parser.c", "src/scanner.c"],
        },
        GrammarSpec {
            library_name: "markdown-inline",
            repo_url: Url::parse("https://github.com/tree-sitter-grammars/tree-sitter-markdown")
                .unwrap(),
            rev: "7462bb66ac7e90312082269007fac2772fe5efd1",
            subdir: "tree-sitter-markdown-inline",
            source: vec!["src/parser.c", "src/scanner.c"],
        },
        GrammarSpec {
            library_name: "json",
            repo_url: Url::parse("https://github.com/tree-sitter/tree-sitter-json.git").unwrap(),
            rev: "ee35a6ebefcef0c5c416c0d1ccec7370cfca5a24",
            subdir: ".",
            source: vec!["src/parser.c"],
        },
    ];

    // Determine path to where the grammars will be cloned
    let out_dir = env::var("OUT_DIR")?;
    let grammars_dir = PathBuf::from(&out_dir);

    for spec in grammar_specs {
        // Extract repository name from URL
        let repo_name = spec
            .repo_url
            .path_segments()
            .and_then(|segments| segments.last())
            .ok_or("Invalid repository URL")?
            .trim_end_matches(".git");
        let repo_dir = grammars_dir.join(repo_name);

        // Clone the repository if it doesn't exist
        if !repo_dir.join("__ok__").exists() {
            let orig_current_dir = env::current_dir()?;
            _ = fs::remove_dir_all(&repo_dir);
            fs::create_dir_all(&repo_dir)?;
            env::set_current_dir(&repo_dir)?;
            exec("git", &["init", "-q"])?;
            exec("git", &["remote", "add", "origin", spec.repo_url.as_str()])?;
            exec("git", &["fetch", "-q", "--depth", "1", "origin", spec.rev])?;
            exec("git", &["checkout", "-q", "FETCH_HEAD"])?;
            fs::write(repo_dir.join("__ok__"), b"")?;
            env::set_current_dir(&orig_current_dir)?;
        }

        // Compile the grammar library using cc
        let include = repo_dir.join(spec.subdir).join("src");
        let source_files: Vec<PathBuf> = spec
            .source
            .iter()
            .map(|p| repo_dir.join(spec.subdir).join(p))
            .collect();
        Build::new()
            .opt_level(3)
            .include(include)
            .files(&source_files)
            .compile(spec.library_name);

        // Set build script to run again if grammars change (extra safety net)
        source_files
            .iter()
            .for_each(|p| println!("cargo::rerun-if-changed={}", p.to_string_lossy()));
    }

    Ok(())
}
