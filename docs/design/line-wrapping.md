# Line Wrapping Specification

This specification defines the observable behavior of the `line_wrapping` configuration option:
when formatting preserves or splits physical lines. Markdown spacing is specified separately in
[spacing.md](spacing.md).

## Configuration

`line_wrapping` accepts `preserve` and `split`. It can be set in `.cjkfmt.json`, with the
`CJKFMT_LINE_WRAPPING` environment variable, or with the global `--line-wrapping` option. The
existing configuration precedence applies: CLI, environment, project configuration, user
configuration, then defaults. The default is `split`; configurations that omit the new setting
retain the existing wrapping behavior. `max_width` remains a separate setting and must be at least 2
in either mode.

## Formatting

Formatting first applies the selected language's configured spacing edits. In `split` mode, it then
uses the existing language-approved safe breaks, width calculation, line terminators, and syntax
safety rules. This mode does not join existing physical lines or promise that every line fits when
no safe break is available.

In `preserve` mode, formatting returns after applying spacing edits. It does not add, remove, or
join line breaks; normalize LF, CRLF, or bare CR endings; change blank lines or final-newline
presence; or rewrite indentation or continuation markers. Markdown spacing rules still apply to
eligible prose. JSON has no spacing edits, so JSON input is byte-identical. If no language is
selected, formatting remains byte-identical in either mode.

## Checking

`check` is independent of `line_wrapping`. W001 measures every original physical input line,
including lines with no safe break and protected Markdown or JSON content. Its diagnostics are the
same in both modes. Spacing diagnostics retain their existing behavior as defined in
[spacing.md](spacing.md).

Formatting is not a guarantee that W001 is clear: preserved lines may remain overlong, and
configured spacing edits may make a line exceed `max_width`. This is not a formatting failure.
