use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(ValueEnum, Debug, Clone, Deserialize, Serialize)]
pub enum ColorOutputMode {
    Always,
    Never,
    Auto,
}

#[derive(Parser, Debug, Deserialize, Serialize)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Check whether formatting is correct without modifying the files.
    #[arg(short, long, default_value = "false")]
    pub check: bool,

    /// Control whether to colorize the output.
    ///
    /// When set to `always`, cjkfmt will always produce colorized output. When set
    /// to `never`, the output will always be plain text without any colors. The
    /// `auto` option enables cjkfmt to decide automatically based on the terminal's
    /// capabilities and environment variables, such as `NO_COLOR` and `CLICOLOR`.
    #[arg(value_enum, long, default_value_t = ColorOutputMode::Auto)]
    pub color: ColorOutputMode,

    /// Maximum line width to allow. [default: 80]
    // Figment handles fallback operation, so this is optional.
    #[arg(short, long)]
    pub max_width: Option<u32>,

    /// File(s) to process.
    #[arg()]
    filenames: Vec<PathBuf>,
}

impl Cli {
    pub fn filenames(&self) -> Vec<&Path> {
        self.filenames
            .iter()
            .map(|p| p.as_path())
            .collect::<Vec<&Path>>()
    }
}
