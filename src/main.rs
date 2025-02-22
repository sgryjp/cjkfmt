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
            filenames,
            max_width,
        } => check_command(filenames, max_width)?,
        args::Commands::Format {
            filenames,
            max_width,
        } => format_command(filenames, max_width)?,
    }
    Ok(())
}
