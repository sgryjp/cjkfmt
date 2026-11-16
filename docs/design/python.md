# Python support

Python support is an initial, comment-only slice. It applies the configured alphabet and digit
spacing rules to safely recognized standalone and trailing comments. It does not format docstrings,
ordinary strings, or executable code, and it never wraps Python lines. The `--line-wrapping` setting
therefore does not change Python source layout.

## Language selection

The final `.py`, `.pyi`, or `.pyw` extension selects Python case-insensitively for ASCII letters.
No other extension implies Python. Use `--language python` for stdin or a file with another
extension; an explicit selection overrides the
filename. `format` with no selected language leaves stdin unchanged, while `check` retains its
existing Markdown default for stdin.

## Syntax and encoding

The bundled Tree-sitter Python grammar is pinned to revision
`26855eabccb19c6abf499fbc5b8dc7cc9ab8bc64` of
[`tree-sitter-python`](https://github.com/tree-sitter/tree-sitter-python). cjkfmt does not invoke a
Python interpreter. Syntax errors and missing syntax nodes reported by this grammar are treated as
operational failures, but successful parsing is not a guarantee that every Python interpreter or
version accepts the source. This revision may recover from incomplete constructs Python rejects; for
example, it can accept `def f():` without a body. Syntax validation is therefore limited to errors
reported by the pinned grammar.

Python input must be UTF-8, with or without a UTF-8 BOM. The BOM is preserved. Invalid UTF-8,
non-UTF-8 encoding declarations in the first two source lines, and conflicting BOM/declaration
combinations are rejected rather than rewritten. Python source remains otherwise byte-preserving:
indentation, comment markers, physical line endings, and source layout are unchanged.

## Comment coverage and limits

Spacing applies only to syntax-recognized Python comments. Ordinary strings and docstrings are not
comments and remain unchanged. A trailing comment is considered independently. Consecutive
standalone comments with matching indentation form one region; blank lines, code, and indentation
changes end that region.

In consecutive standalone-comment regions, cjkfmt protects Markdown fenced code blocks (backtick
or tilde fences), reStructuredText code and code-block directives, and indented literal blocks
introduced by `::`, as well as doctest prompts, continuations, and
output. Safe prose before and after a protected block remains eligible
for spacing. If a block is incomplete or its end cannot be
determined safely, the remainder of that comment region is left unchanged. Trailing comments are
considered independently.

Other recognizable markup, including inline backticks, URLs, and link syntax, causes cjkfmt to skip
the remainder of the comment region from that line onward when it cannot determine a safe boundary.
This is not a complete Markdown or reStructuredText parser. Unmarked code-like text and
machine-directed comments are not reliably detected and may be formatted; explicitly mark code
examples to keep them unchanged. cjkfmt does not maintain a blacklist of tool-specific comment
directives.

`check` derives W002 from the same validated spacing plan used by `format`, so reported locations
refer to the original source. W001 checks physical lines independently of spacing and wrapping. A
readable Python file with syntax errors can still report W001, but it reports no W002 and the
command fails operationally. Undecodable input is rejected before diagnostics are calculated.

This initial scope does not complete the broader Python comment and docstring formatting feature.
