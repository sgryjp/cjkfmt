<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## v0.0.7 - 2025-08-05

### Added

- Added functionality to configure how characters in the Ambiguous category are treated:
  as either narrow (half-width) or wide (full-width).
  - cjkfmt follows the East Asian Width property as specified in
    [Unicode Standard Annex #11](https://www.unicode.org/reports/tr11/),
    using the [`unicode-width`](https://crates.io/crates/unicode-width) crate.

- Functionality to configure spacing rules.
  - `spacing.alphabets` controls how to handle spaces between full-width and half-width alphabets.
    Possible values are `require`, `prohibit`, and `ignore`.
  - `spacing.digits` controls how to handle spaces between full-width and half-width digits.
    Possible values are the same as for alphabets.
  - `spacing.punctuation_as_fullwidth` controls whether to treat full-width punctuation as full-width
    characters or not.

### Fix

- Do not recognize full-width punctuation marks as full-width characters

## v0.0.6 - 2025-07-09

### Added

- Functionality to check spacing a full-width character and a half-width character.
- Support for processing files with CR+LF line endings.

## v0.0.5 - 2025-06-24

### Added

- Command line option `--color` to control whether to use colorized output or not.

## v0.0.4 - 2025-06-23

### Added

- Layered configuration support

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

- Basic western word wrapping and kinsoku rule
