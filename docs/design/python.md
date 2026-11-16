# Python support

Python support applies the configured alphabet and digit spacing rules to safely recognized
standalone and trailing comments, plus actual module, class, and synchronous or asynchronous
function docstrings. It does not format ordinary strings or executable code, and it never wraps
Python lines. The `--line-wrapping` setting therefore does not change Python source layout.

## Language selection

The final `.py`, `.pyi`, or `.pyw` extension selects Python case-insensitively for ASCII letters.
No other extension implies Python. Use `--language python` for stdin or a file with another
extension; an explicit selection overrides the filename. `format` with no selected language leaves
stdin unchanged, while `check` retains its existing Markdown default for stdin.

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

## Python prose coverage and limits

Spacing applies to syntax-recognized Python comments and actual docstrings. A trailing comment is
considered independently. Consecutive standalone comments with matching indentation form one region;
blank lines, code, and indentation changes end that region.

In consecutive standalone-comment regions, cjkfmt protects Markdown fenced code blocks (backtick
or tilde fences), reStructuredText code and code-block directives, indented literal blocks
introduced by `::`, and doctest prompts, continuations, and output. Safe prose before and after a
protected block remains eligible for spacing. If a block is incomplete or its end cannot be
determined safely, the remainder of that comment region is left unchanged. Trailing comments are
considered independently.

Inline code spans and URLs are protected, while surrounding prose and inline link display text
remain eligible for spacing. When cjkfmt cannot determine a safe boundary for inline or other
recognizable markup, it leaves the remainder of the comment region unchanged. This is not a
complete Markdown or reStructuredText parser. Unmarked code-like text and machine-directed comments
are not reliably detected and may be formatted; explicitly mark code examples to keep them
unchanged. cjkfmt does not maintain a blacklist of tool-specific comment directives.

A docstring must be the first statement in its module, class, or synchronous or asynchronous
function. Comments do not count as statements. Parenthesized and implicitly concatenated ordinary
strings, including optional `u` and `r` prefixes, are supported. Each literal is processed
independently. Spacing decisions do not cross literal boundaries or escape sequences. Literal
spelling, escapes, quotes, prefixes, indentation, and line endings are preserved.

Byte strings, f-strings, binary expressions, ordinary strings, attribute docstrings, and additional
docstrings are not formatted. Docstring-specific protected-markup handling is not yet implemented,
so cjkfmt conservatively skips a whole docstring containing common markup indicators such as
backticks, URL or link syntax, code fences, reST literal/directive markers, or doctest prompts. This
is not a complete Markdown or reStructuredText parser.

`check` derives W002 from the same validated spacing plan used by `format`, so reported locations
refer to the original source. W001 checks physical lines independently of spacing and wrapping. A
readable Python file with syntax errors can still report W001, but it reports no W002 and the
command fails operationally. Undecodable input is rejected before diagnostics are calculated.

Docstrings containing protected markup are skipped as a whole. Comment regions use safe source
ranges around recognized markup and fail closed when a boundary is ambiguous.
