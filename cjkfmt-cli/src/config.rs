use std::{env, path::PathBuf};

use clap::ValueEnum;
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

    /// Maximum line width to allow. (default: 80)
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
            .filter(|p| p.exists());

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
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
    // /// Whether to treat full-width punctuation as full-width characters or not.
    // pub punctuation_as_fullwidth: bool, // TODO: Implement this option
}

impl Default for SpacingConfig {
    fn default() -> Self {
        SpacingConfig {
            alphabets: SpacingRule::Ignore,
            digits: SpacingRule::Ignore,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use figment::{
        Figment,
        providers::{Format, Json, Serialized},
    };
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    fn parse_args(arguments: impl IntoIterator<Item = &'static str>) -> CliArgs {
        CliArgs::try_parse_from(arguments).expect("the command-line arguments should parse")
    }

    #[test]
    // Re-implements `Config::from_cli_args`'s merge chain rather than calling it, since
    // that function touches real files and env vars. Keep the two in sync manually.
    fn configuration_sources_are_applied_in_default_file_env_cli_order() {
        let args = parse_args(["cjkfmt", "--spacing-digits", "ignore", "format"]);
        // Stands in for the env layer's position in the chain; not real `Env::prefixed` parsing.
        let environment = json!({
            "spacing": {
                "alphabets": "prohibit",
                "digits": "prohibit",
            },
        });
        let config: Config = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Json::string(
                r#"{
                    "max_width": 90,
                    "ambiguous_width": "narrow",
                    "spacing": { "alphabets": "require", "digits": "require" }
                }"#,
            ))
            .merge(Serialized::defaults(environment))
            .merge(&args)
            .extract()
            .expect("all configuration sources should deserialize");

        assert_eq!(config.max_width, 90, "the file should override the default");
        assert_eq!(
            config.ambiguous_width,
            AmbiguousWidth::Narrow,
            "the file should override the default"
        );
        assert_eq!(
            config.spacing.alphabets,
            SpacingRule::Prohibit,
            "the environment should override the file"
        );
        assert_eq!(
            config.spacing.digits,
            SpacingRule::Ignore,
            "the CLI should override the environment"
        );
    }

    #[rstest]
    // "snake_case" is the documented, canonical form (ADR-0001).
    #[case("narrow", Some(AmbiguousWidth::Narrow))]
    #[case("wide", Some(AmbiguousWidth::Wide))]
    #[case("halfwidth", Some(AmbiguousWidth::Narrow))]
    #[case("fullwidth", Some(AmbiguousWidth::Wide))]
    #[case("Narrow", None)]
    #[case("Wide", None)]
    #[case("Halfwidth", None)]
    #[case("Fullwidth", None)]
    fn ambiguous_width_accepts_only_snake_case_value(
        #[case] value: &str,
        #[case] expected: Option<AmbiguousWidth>,
    ) {
        let result: Result<Config, _> = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Json::string(&format!(
                r#"{{ "ambiguous_width": "{value}" }}"#
            )))
            .extract();

        match expected {
            Some(expected) => assert_eq!(
                result
                    .expect("the documented snake_case value should deserialize")
                    .ambiguous_width,
                expected
            ),
            None => assert!(result.is_err(), "non-snake_case value should be rejected"),
        }
    }

    #[rstest]
    // "snake_case" is the documented, canonical form (ADR-0001).
    #[case("require", Some(SpacingRule::Require))]
    #[case("prohibit", Some(SpacingRule::Prohibit))]
    #[case("ignore", Some(SpacingRule::Ignore))]
    #[case("Require", None)]
    #[case("Prohibit", None)]
    #[case("Ignore", None)]
    fn spacing_rule_accepts_only_snake_case_value(
        #[case] value: &str,
        #[case] expected: Option<SpacingRule>,
    ) {
        let result: Result<Config, _> = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Json::string(&format!(
                r#"{{ "spacing": {{ "alphabets": "{value}" }} }}"#
            )))
            .extract();

        match expected {
            Some(expected) => assert_eq!(
                result
                    .expect("the documented snake_case value should deserialize")
                    .spacing
                    .alphabets,
                expected
            ),
            None => assert!(result.is_err(), "non-snake_case value should be rejected"),
        }
    }
}

/// How to treat width of characters in the Ambiguous category according to Unicode Standard Annex #11.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguousWidth {
    /// Treat characters in the Ambiguous category as 1.
    // `halfwidth` is kept as a friendlier synonym some users may reach for.
    #[serde(alias = "halfwidth")]
    Narrow,

    /// Treat characters in the Ambiguous category as 2.
    #[serde(alias = "fullwidth")]
    Wide,
}
