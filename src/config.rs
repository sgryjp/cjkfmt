use std::{env, path::PathBuf};

use figment::{
    Figment,
    providers::{Env, Format, Json, Serialized},
};
use serde::{Deserialize, Serialize};

use crate::args::CliArgs;

/// The configuration for cjkfmt.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// How to treat width of characters in the Ambiguous category according to Unicode Standard Annex #11.
    pub ambiguous_width: AmbiguousWidth,

    /// Maximum line width to allow. (dfault: 80)
    pub max_width: u32,

    /// Rules for handling spaces between full-width and half-width characters.
    pub spacing: SpacingConfig,
}

impl Config {
    pub fn from_cli_args(args: &CliArgs) -> Result<Self, Box<figment::Error>> {
        // Resolve configuration directory.
        // XDG_CONFIG_HOME is used if set, otherwise defaults to $HOME.
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(env::home_dir);
        let user_config_path = config_home
            .map(|p| p.join(".cjkfmt.json"))
            .and_then(|p| if p.exists() { Some(p) } else { None });

        // Load configuration from various sources:
        //
        // 1. Default values
        // 2. JSON file `.cjkfmt.json` at the user's configuration directory
        //    (`XDG_CONFIG_HOME` if set, otherwise `$HOME/.config`)
        // 3. JSON file `.cjkfmt.json` found in the current or ancestor directories
        // 4. Environment variables prefixed with `CJKFMT_`
        let config = Figment::new();
        let config = config.merge(Serialized::defaults(Config::default()));
        let config = user_config_path.map_or(config.clone(), |p| config.merge(Json::file_exact(p)));
        let config = config.merge(Json::file(".cjkfmt.json"));
        let config = config.merge(Env::prefixed("CJKFMT_"));
        let config = config.merge(args);
        let config: Self = config.extract()?;

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ambiguous_width: AmbiguousWidth::Wide,
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

/// How to treat width of characters in the Ambiguous category according to Unicode Standard Annex #11.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmbiguousWidth {
    /// Treat characters in the Ambiguous category as 1.
    #[serde(alias = "Halfwidth")]
    Narrow,

    /// Treat characters in the Ambiguous category as 2.
    #[serde(alias = "Fullwidth")]
    Wide,
}
