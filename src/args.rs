use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Check whether formatting is correct without modifying the files.
    #[arg(short, long, default_value = "false")]
    pub check: bool,

    /// Maximum line width to allow.
    #[arg(short, long)]
    pub max_width: u32,

    /// File(s) to process.
    #[arg()]
    pub filenames: Vec<PathBuf>,
}
