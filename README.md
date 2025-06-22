# cjkfmt

[![Crates.io](https://img.shields.io/crates/v/cjkfmt.svg)](https://crates.io/crates/cjkfmt)
[![CI](https://github.com/sgryjp/cjkfmt/workflows/CI/badge.svg)](https://github.com/sgryjp/cjkfmt/actions)

<!-- [![Docs.rs](https://docs.rs/cjkfmt/badge.svg)](https://docs.rs/cjkfmt) -->

## Installation

### Cargo

- Install the rust toolchain in order to have cargo installed by following
  [this](https://www.rust-lang.org/tools/install) guide.
- run `cargo install cjkfmt`

## Configuration

cjkfmt can be configured in several ways, with configuration options applied in the following order of precedence (highest to lowest):

1. Command line options
2. Environment variables prefixed with `CJKFMT_`
3. JSON configuration file `.cjkfmt.json` found in the current or ancestor directories
4. JSON configuration file `.cjkfmt.json` in the user's configuration directory
   (`XDG_CONFIG_HOME` if set, otherwise `$HOME`)
5. Default values

The configuration file is searched for in the current directory and, if not found, in each parent directory up to the root. Only the first file found will be used.

### Configuration Options

Currently, the following configuration options are available:

| Option      | Description                 | Default |
| ----------- | --------------------------- | ------- |
| `max_width` | Maximum line width to allow | 80      |

Depending on the configuration source, the option names are formatted slightly differently:

- JSON configuration files
  - Use underscores between words.
    Example: `max_width`
- Environment variables
  - Write names in ALL CAPITAL LETTERS with underscores between words,
    and always start with `CJKFMT_`.
    Example: `CJKFMT_MAX_WIDTH`
- Command line options
  - Use hyphens between words, and put two dashes before the option name.
    Example: `--max-width 100`

### Example Configuration File

Below is an example configuration file `.cjkfmt.json`.

```json
{
  "max_width": 100
}
```

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
