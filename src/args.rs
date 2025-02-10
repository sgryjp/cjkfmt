use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// File to process.
    #[arg()]
    pub filename: Option<PathBuf>,

    /// Maximum line width to allow.
    #[arg(short, long)]
    pub max_width: usize,
}
