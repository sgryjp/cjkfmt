use std::{collections::BTreeMap, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use figment::{
    Profile, Provider,
    value::{Dict, Map, Value},
};
use serde::{Deserialize, Serialize};

use crate::config::{AmbiguousWidth, SpacingRule};

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

    /// How to treat characters in Unicode's Ambiguous category: `narrow` or `wide`. [default: wide]
    #[arg(long, value_enum)]
    pub ambiguous_width: Option<AmbiguousWidth>,

    /// Require, prohibit, or ignore spaces between full-width and half-width alphabets. [default: ignore]
    #[arg(long, value_enum)]
    pub spacing_alphabets: Option<SpacingRule>,

    /// Require, prohibit, or ignore spaces between full-width and half-width digits. [default: ignore]
    #[arg(long, value_enum)]
    pub spacing_digits: Option<SpacingRule>,

    /// Whether full-width punctuation is treated as full-width (`true` or `false`). [default: false]
    #[arg(long, action = clap::ArgAction::Set)]
    pub spacing_punctuation_as_fullwidth: Option<bool>,

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
            dict.insert("max_width".to_string(), Value::from(max_width));
        }
        if let Some(ambiguous_width) = self.ambiguous_width {
            dict.insert(
                "ambiguous_width".to_string(),
                Value::from(format!("{ambiguous_width:?}")),
            );
        }

        let mut spacing = BTreeMap::new();
        if let Some(alphabets) = self.spacing_alphabets {
            spacing.insert(
                "alphabets".to_string(),
                Value::from(format!("{alphabets:?}").to_ascii_lowercase()),
            );
        }
        if let Some(digits) = self.spacing_digits {
            spacing.insert(
                "digits".to_string(),
                Value::from(format!("{digits:?}").to_ascii_lowercase()),
            );
        }
        if let Some(punctuation_as_fullwidth) = self.spacing_punctuation_as_fullwidth {
            spacing.insert(
                "punctuation_as_fullwidth".to_string(),
                Value::from(punctuation_as_fullwidth),
            );
        }
        if !spacing.is_empty() {
            dict.insert("spacing".to_string(), Value::from(spacing));
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

    /// Print the parsed concrete syntax tree for debugging.
    DebugCst {
        /// File(s) to process.
        #[arg()]
        filenames: Vec<PathBuf>,
    },
}
