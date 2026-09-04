# Name explicit input selection `--language`

Status: accepted

The `check`, `format`, and `debug-cst` commands expose explicit input selection as
`--language`, with the `markdown` and `json` values, rather than `--grammar`, `--parser`, or
`--type`. Users choose the kind of document they are processing, not cjkfmt's internal tree-sitter
implementation; cjkfmt currently has one parser grammar per supported language, so exposing
grammar or parser would leak an implementation detail without exposing a meaningful choice.

## Considered Options

- **`--grammar`.** Rejected because it exposes the internal tree-sitter grammar rather than the
  document kind the user intends.
- **`--parser`.** Rejected because cjkfmt does not offer a user-relevant choice among parsers for
  one language.
- **`--type`.** Rejected because it is less precise and could mean unrelated classifications, such
  as a MIME type.
