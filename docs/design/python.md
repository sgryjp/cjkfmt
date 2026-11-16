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

## Comment coverage and limits

Spacing applies to syntax-recognized Python comments and actual docstrings. A trailing comment is
considered independently. Consecutive standalone comments with matching indentation form one region;
blank lines, code, and indentation changes end that region.

A docstring must be the first statement in its module, class, or synchronous or asynchronous
function. Comments do not count as statements. Parenthesized and implicitly concatenated ordinary
strings, including optional `u` and `r` prefixes, are supported. Each literal is processed
independently. Spacing decisions do not cross literal boundaries or escape sequences. Literal
spelling, escapes, quotes, prefixes, indentation, and line endings are preserved.

Byte strings, f-strings, binary expressions, ordinary strings, attribute docstrings, and additional
docstrings are not formatted. Until protected-markup support is implemented, cjkfmt conservatively
skips a whole docstring containing common markup indicators such as backticks, URL or link syntax,
code fences, reST literal/directive markers, or doctest prompts. It likewise skips an entire comment
region when it sees those indicators, avoiding partial changes to marked examples split across
consecutive comments. Detection is intentionally conservative, not a complete Markdown or
reStructuredText parser. Unmarked code-like text and machine-directed comments are not reliably
detected and may be formatted; explicitly mark examples to keep them unchanged.

`check` derives W002 from the same validated spacing plan used by `format`, so reported locations
refer to the original source. W001 checks physical lines independently of spacing and wrapping. A
readable Python file with syntax errors can still report W001, but it reports no W002 and the
command fails operationally. Undecodable input is rejected before diagnostics are calculated.

Protected markup is handled conservatively at whole-docstring or whole-comment-region granularity;
safe subregions adjacent to protected markup are not formatted yet.
