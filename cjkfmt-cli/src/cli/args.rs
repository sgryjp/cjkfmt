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
                Value::serialize(ambiguous_width)?,
            );
        }

        let mut spacing = BTreeMap::new();
        if let Some(alphabets) = self.spacing_alphabets {
            spacing.insert("alphabets".to_string(), Value::serialize(alphabets)?);
        }
        if let Some(digits) = self.spacing_digits {
            spacing.insert("digits".to_string(), Value::serialize(digits)?);
        }
        if !spacing.is_empty() {
            dict.insert("spacing".to_string(), Value::from(spacing));
        }

        let mut map = BTreeMap::new();
        map.insert(Profile::Default, dict);

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use figment::{Figment, providers::Serialized};
    use rstest::rstest;

    use super::*;
    use crate::config::Config;

    fn config_from(arguments: impl IntoIterator<Item = &'static str>) -> Config {
        let args =
            CliArgs::try_parse_from(arguments).expect("the command-line arguments should parse");

        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(&args)
            .extract()
            .expect("CLI values should deserialize as configuration")
    }

    #[rstest]
    #[case("--write")]
    #[case("-w")]
    fn format_write_flag_accepts_a_filename(#[case] flag: &str) {
        let args = CliArgs::try_parse_from(["cjkfmt", "format", flag, "file.md"])
            .expect("the write command-line arguments should parse");

        match args.command {
            Commands::Format { write, filenames } => {
                assert!(write);
                assert_eq!(filenames, [PathBuf::from("file.md")]);
            }
            _ => panic!("expected format command"),
        }
    }

    #[test]
    fn format_write_flag_requires_a_filename() {
        let result = CliArgs::try_parse_from(["cjkfmt", "format", "--write"]);

        assert!(result.is_err());
    }

    #[test]
    fn max_width_flag_maps_clap_value_to_config() {
        let config = config_from(["cjkfmt", "--max-width", "42", "format"]);

        assert_eq!(config.max_width, 42);
    }

    #[rstest]
    #[case("narrow", AmbiguousWidth::Narrow)]
    #[case("wide", AmbiguousWidth::Wide)]
    fn ambiguous_width_flag_maps_each_clap_value_to_config(
        #[case] value: &'static str,
        #[case] expected: AmbiguousWidth,
    ) {
        let config = config_from(["cjkfmt", "--ambiguous-width", value, "format"]);

        assert_eq!(config.ambiguous_width, expected);
    }

    #[rstest]
    #[case("require", SpacingRule::Require)]
    #[case("prohibit", SpacingRule::Prohibit)]
    #[case("ignore", SpacingRule::Ignore)]
    fn spacing_alphabets_flag_maps_each_clap_value_to_config(
        #[case] value: &'static str,
        #[case] expected: SpacingRule,
    ) {
        let config = config_from(["cjkfmt", "--spacing-alphabets", value, "format"]);

        assert_eq!(config.spacing.alphabets, expected);
    }

    #[rstest]
    #[case("require", SpacingRule::Require)]
    #[case("prohibit", SpacingRule::Prohibit)]
    #[case("ignore", SpacingRule::Ignore)]
    fn spacing_digits_flag_maps_each_clap_value_to_config(
        #[case] value: &'static str,
        #[case] expected: SpacingRule,
    ) {
        let config = config_from(["cjkfmt", "--spacing-digits", value, "format"]);

        assert_eq!(config.spacing.digits, expected);
    }

    #[test]
    fn spacing_flags_are_merged_as_independent_nested_config_values() {
        let config = config_from([
            "cjkfmt",
            "--spacing-alphabets",
            "require",
            "--spacing-digits",
            "prohibit",
            "format",
        ]);

        assert_eq!(config.spacing.alphabets, SpacingRule::Require);
        assert_eq!(config.spacing.digits, SpacingRule::Prohibit);
    }
}

#[derive(Subcommand, Debug, Deserialize, Serialize)]
pub enum Commands {
    /// Format files according to CJK text formatting rules.
    Format {
        /// Replace each input file with its formatted content instead of writing to stdout.
        #[arg(short, long, requires = "filenames")]
        write: bool,

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
