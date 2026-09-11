<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## v0.0.7 - 2026-09-11

### Added

- Added syntax-aware Markdown support. (#4)
- Added `cjkfmt debug-cst` to print parsed concrete syntax trees for debugging. (#81, #83)
- Added the `--language markdown|json` option to `format`, `check`, and `debug-cst` to override
  filename-based language detection. (#82, #102)
- Added `cjkfmt format --write` to replace named files with their formatted content. (#94, #95)
- `cjkfmt format` now applies configured CJK/ASCII spacing rules to Markdown prose. (#5, #90)
- Added the CLI options for config override: `--ambiguous-width`, `--spacing-alphabets`,
  `--spacing-digits`, and `--spacing-punctuation-as-fullwidth`. (#87)
- Added configuration for treating East Asian Width Ambiguous characters as narrow (half-width) or
  wide (full-width), following [Unicode Standard Annex #11](https://www.unicode.org/reports/tr11/)
  via the [`unicode-width`](https://crates.io/crates/unicode-width) crate. (#20)
- Added configurable spacing rules:
  - `spacing.alphabets` controls spaces between full-width and half-width alphabets.
  - `spacing.digits` controls spaces between full-width and half-width digits.
  - Both settings accept `require`, `prohibit`, or `ignore`.

### Changed

- Canonicalized filename-based language selection across `format`, `check`, and `debug-cst`.
  Unsupported named paths are now rejected by `check` and `debug-cst` unless
  `--language markdown|json` is used. (#96, #103)

### Fixed

- Preserve each source line's line-ending style when inserting wrapping breaks, including mixed LF
  and CRLF input. (#105, #109)
- Aligned Markdown spacing checks with formatting so excluded inline content is not diagnosed.
  (#91, #101)
- Fixed spacing diagnostic columns for inline Markdown content that begins after the start of a
  line. (#62, #63, #64, #74)
- Do not recognize full-width punctuation marks as full-width characters.

### Miscellaneous

- Centralized Markdown prose spacing edit planning for use by formatting and future checking.
  (#91, #100)
- Refactored the space problem checker to work on the concrete syntax tree (CST) rather than
  processing plain text line by line. (#31)

## v0.0.6 - 2025-07-09

### Added

- Functionality to check spacing a full-width character and a half-width character.
- Support for processing files with CR+LF line endings.

## v0.0.5 - 2025-06-24

### Added

- Command line option `--color` to control whether to use colorized output or not.

## v0.0.4 - 2025-06-23

### Added

- Layered configuration support. (#3)

### Fixed

- Diagnostic messages in check mode are now correctly output to stdout instead of stderr.

## v0.0.3 - 2025-06-21

### Added

- Spacing check functionality to `check` mode (experimental)

## v0.0.2 - 2025-06-19

### Fixed

- Diagnostic position in `check` mode now correctly points to the overflow position instead of the
  wrap position.

## v0.0.1 - 2025-06-09

### Added

- Basic western word wrapping and kinsoku rule. (#2)
