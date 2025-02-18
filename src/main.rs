mod args;
mod check;
mod diagnostic;
mod format;
mod line_break;

use clap::Parser as _;

use crate::{args::Cli, check::check_command, format::format_command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        args::Commands::Check {
            filename,
            max_width,
        } => check_command(filename.as_deref(), max_width)?,
        args::Commands::Format {
            filename,
            max_width,
        } => format_command(filename.as_deref(), max_width)?,
    }
    Ok(())
}
