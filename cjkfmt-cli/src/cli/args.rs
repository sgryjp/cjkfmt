use std::{collections::BTreeMap, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use figment::{
    Profile, Provider,
    value::{Dict, Map},
};
use serde::{Deserialize, Serialize};

#[derive(ValueEnum, Debug, Clone, Deserialize, Serialize)]
pub enum ColorOutputMode {
    Always,
    Never,
    Auto,
}

#[derive(Parser, Debug, Deserialize, Serialize)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
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

    #[command(subcommand)]
    pub command: Commands,
}

// Implementing the Provider trait for CliArgs to integrate with Figment
impl Provider for CliArgs {
    fn metadata(&self) -> figment::Metadata {
        figment::Metadata::named("Command line arguments")
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let mut dict = BTreeMap::new();
        if let Some(max_width) = self.max_width {
            dict.insert(
                "max_width".to_string(),
                figment::value::Value::from(max_width),
            );
        }

        let mut map = BTreeMap::new();
        map.insert(Profile::Default, dict);

        Ok(map)
    }
}

#[derive(Subcommand, Debug, Deserialize, Serialize)]
pub enum Commands {
    /// Format files according to CJK text formatting rules.
    Format {
        /// File(s) to process.
        #[arg()]
        filenames: Vec<PathBuf>,
    },

    /// Check whether formatting is correct without modifying the files.
    Check {
        /// File(s) to process.
        #[arg()]
        filenames: Vec<PathBuf>,
    },
}
