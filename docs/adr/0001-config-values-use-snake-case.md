# Config values are serialized in snake_case

Status: accepted

cjkfmt's configuration file keys are already snake_case, matching the Rust
struct field names they map to (e.g. `ambiguous_width`, `spacing.alphabets`).
We serialize/deserialize the _values_ of config enums the same way — using
`#[serde(rename_all = "snake_case")]` — rather than leaving them as their Rust
variant names (PascalCase) or switching to kebab-case, so that every config
file uses one consistent word-separator convention throughout, for both keys
and values.

## Considered Options

- **PascalCase (Rust variant names as-is).** Rejected: it leaks an
  implementation detail (Rust's own naming convention for enum variants)
  into the config file, and mismatches the snake_case keys sitting right
  next to it in the same document.
- **kebab-case.** Common among CLI tools' config files, but rejected here:
  config keys are already snake_case for free (they're just the Rust field
  names), so giving values the same treatment via `rename_all` costs
  nothing extra. kebab-case would require explicit conversion everywhere
  and would mix two different separator styles in one file.

---

Note: this ADR was recorded retrospectively on 2026-08-12. The convention
itself originates from `af843aa` ("feat: make spacing rules configurable",
2025-07-13), where `SpacingRule` first adopted
`#[serde(rename_all = "snake_case")]`. It should have been applied
consistently to every config enum added from that point onward.
