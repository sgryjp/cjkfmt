use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check whether too long line exists or not.
    Check {},

    /// Wrap long lines with adherence to kinsoku rule.
    Format {
        /// File to process.
        #[arg()]
        filename: Option<PathBuf>,

        /// Maximum line width to allow.
        #[arg(short, long)]
        max_width: usize,
    },
}
