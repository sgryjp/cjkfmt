# cjkfmt

cjkfmt formats CJK prose while leaving programming and markup syntax unchanged.

## Language

**Formatting range**:
A byte range in a source document whose prose is eligible for configured spacing rules.
_Avoid_: editable text, target text

**Python formatting content**:
The comment body and literal text of grammatical docstrings eligible for spacing formatting;
escape sequences and boundaries between adjacent literals are excluded,
while embedded code examples are included.
_Avoid_: Python prose, Python text, string content
