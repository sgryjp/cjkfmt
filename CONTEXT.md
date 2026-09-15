# cjkfmt

cjkfmt checks and formats text with CJK-aware line width and spacing rules.

## Language

**CJK-aware formatting**:
Source formatting based on CJK-aware spacing rules and physical source-line width. The selected
Language constrains where changes may occur, but language-specific code layout and style are not
reformatted.
_Avoid_: Language formatting, code formatting

**Language**:
The user-facing kind of input document selected with `--language`. The supported canonical
values are lowercase `markdown` and `json`; an explicit selection takes precedence over
filename-based detection. Formatting preserves the input language rather than converting it to
another language.
JSON selection disables Markdown-prose spacing edits, but retains cjkfmt's existing general
line-wrapping pass.
_Avoid_: Grammar, parser

**Markdown prose**:
Visible Markdown content eligible for CJK/ASCII spacing rules, including ordinary text, emphasis,
strikethrough, link text, and image descriptions. Code, link metadata, autolinks, HTML, entities,
escaped syntax, and unsafe constructs are not Markdown prose.
_Avoid_: Markdown text, inline content
