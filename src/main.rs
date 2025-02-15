mod args;
mod format;
mod line_break;

use crate::args::Cli;
use clap::Parser as _;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        args::Commands::Check {} => todo!(),
        args::Commands::Format {
            filename,
            max_width,
        } => crate::format::format_command(filename.as_ref(), max_width)?,
    }
    Ok(())
}
