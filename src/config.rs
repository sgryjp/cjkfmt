use std::{env, path::PathBuf};

use figment::{
    Figment,
    providers::{Env, Format, Json, Serialized},
};
use serde::{Deserialize, Serialize};

use crate::args::Cli;

/// The configuration for cjkfmt.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Maximum line width to allow. (default: 80)
    pub max_width: u32,

    /// Rules for handling spaces between full-width and half-width characters.
    pub spacing: SpacingConfig,
}

impl Config {
    pub fn from_cli_args(args: &Cli) -> Result<Self, Box<figment::Error>> {
        // Resolve configuration directory.
        // XDG_CONFIG_HOME is used if set, otherwise defaults to $HOME.
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(env::home_dir);
        let user_config_path = config_home.map(|p| p.join(".cjkfmt.json"));

        // Load configuration from various sources:
        // 1. Default values
        // 2. JSON file `.cjkfmt.json` at the user's configuration directory
        //    (`XDG_CONFIG_HOME` if set, otherwise `$HOME/.config`)
        // 3. JSON file `.cjkfmt.json` found in the current or ancestor directories
        // 4. Environment variables prefixed with `CJKFMT_`
        let config = Figment::new();
        let config = config.merge(Serialized::defaults(Config::default()));
        let config = if let Some(path) = user_config_path {
            if path.exists() {
                config.merge(Json::file_exact(path))
            } else {
                config
            }
        } else {
            config
        };
        let config = config.merge(Json::file(".cjkfmt.json"));
        let config = config.merge(Env::prefixed("CJKFMT_"));
        let mut config: Self = config.extract()?;

        // Override with command line arguments
        if let Some(max_width) = args.max_width {
            config.max_width = max_width;
        }

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_width: 80,
            spacing: Default::default(),
        }
    }
}

/// Rules for handling spaces between full-width and half-width characters.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpacingRule {
    /// Require a space between full-width and half-width characters.
    Require,

    /// Prohibit spaces between full-width and half-width characters.
    Prohibit,

    /// Do not care about spaces between full-width and half-width characters.
    Ignore,
}

/// Configuration for spacing rules.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SpacingConfig {
    /// How to handle spaces between full-width and half-width alphabets.
    pub alphabets: SpacingRule,

    /// How to handle spaces between full-width and half-width digits.
    pub digits: SpacingRule,

    /// Whether to treat full-width punctuation as full-width characters or not.
    pub punctuation_as_fullwidth: bool, // TODO: Use punctuation_as_fullwidth setting
}

impl Default for SpacingConfig {
    fn default() -> Self {
        SpacingConfig {
            alphabets: SpacingRule::Ignore,
            digits: SpacingRule::Ignore,
            punctuation_as_fullwidth: false,
        }
    }
}
