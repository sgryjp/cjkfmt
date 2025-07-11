<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

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
