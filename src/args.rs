use std::path::{Path, PathBuf};

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Deserialize, Serialize)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Check whether formatting is correct without modifying the files.
    #[arg(short, long, default_value = "false")]
    pub check: bool,

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
