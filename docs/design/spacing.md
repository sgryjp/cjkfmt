---
created: 2028-08-16
---

# CJK/ASCII Spacing Specification

This specification defines how cjkfmt handles ASCII spacing between CJK characters, Latin
characters, and digits. It defines observable behavior rather than a particular parser, data
structure, or edit-application strategy.

The terms **CJK**, **Latin**, and **Digit** in this document are cjkfmt categories. They are not
synonyms for Unicode Script or East Asian Width properties.

## Scope

This specification covers the following behavior:

- classifying Unicode scalar values into cjkfmt character categories;
- deciding whether ASCII spaces are required, prohibited, or ignored between adjacent categories;
- applying those rules to Markdown prose; and
- reporting the same Markdown prose violations through `check` that `format` can correct.

It does not define general whitespace normalization, spacing between Latin characters and digits,
or spacing behavior for source-code languages and other document formats.

## Character categories

Each Unicode scalar value belongs to exactly one of the following categories. Classification is
per scalar value, not per grapheme cluster.

### CJK

A scalar value in one of these candidate ranges is CJK unless its Unicode General Category is one
of the punctuation categories `Pc`, `Pd`, `Pe`, `Pf`, `Pi`, `Po`, or `Ps`. Punctuation in a
candidate range is Other instead.

| Block | Unicode range |
| --- | --- |
| CJK Unified Ideographs | U+4E00–U+9FFF |
| CJK Unified Ideographs Extension A | U+3400–U+4DBF |
| Extension B | U+20000–U+2A6DF |
| Extension C | U+2A700–U+2B73F |
| Extension D | U+2B740–U+2B81F |
| Extension E | U+2B820–U+2CEAF |
| Extension F | U+2CEB0–U+2EBEF |
| Extension G | U+30000–U+3134F |
| Extension H | U+31350–U+323AF |
| Extension I | U+2EBF0–U+2EE5D |
| CJK Radicals Supplement | U+2E80–U+2EFF |
| CJK Symbols and Punctuation | U+3000–U+303F |
| Hiragana | U+3040–U+309F |
| Katakana | U+30A0–U+30FF |
| Bopomofo | U+3100–U+312F |
| Hangul Syllables | U+AC00–U+D7AF |

For example, `漢`, `あ`, `ア`, `ㄅ`, and `가` are CJK. `。`, `《`, `》`, and `・` are Other because
they are punctuation. CJK-related blocks absent from this table, such as CJK Compatibility
Ideographs, Hangul Jamo, and Halfwidth and Fullwidth Forms, are not CJK.

### Latin

The following ranges are Latin. This is a range-based category rather than a Unicode Script test.
No General Category exception applies to these ranges.

| Block | Unicode range |
| --- | --- |
| Basic Latin uppercase and lowercase letters only | U+0041–U+005A, U+0061–U+007A |
| Latin-1 Supplement | U+00C0–U+00FF |
| Latin Extended-A / B / Additional | U+0100–U+017F, U+0180–U+024F, U+1E00–U+1EFF |
| IPA Extensions | U+0250–U+02AF |
| Spacing Modifier Letters | U+02B0–U+02FF |
| Combining Diacritical Marks | U+0300–U+036F |
| Combining Diacritical Marks Extended / Supplement | U+1AB0–U+1AFF, U+1DC0–U+1DFF |
| Latin Extended-C / D / E | U+2C60–U+2C7F, U+A720–U+A7FF, U+AB30–U+AB6F |
| Latin Extended-F / G | U+10780–U+107BF, U+1DF00–U+1DFFF |

Consequently, ASCII punctuation is not Latin, while `×` and `÷` are Latin because they are in the
Latin-1 Supplement range. Combining marks in the listed ranges are also Latin and are classified
independently of their surrounding scalar values.

### Digit

Digit consists only of U+0030–U+0039, the ASCII characters `0`–`9`. Full-width digits `０`–`９`,
CJK numerals, and other Unicode decimal digits are not Digit.

### Whitespace

U+0020 SPACE, U+000D CR, and U+000A LF are spacing boundaries. They are not eligible members of a
spacing pair. Of those values, only U+0020 is editable by this specification.

All other Unicode whitespace is Other. This includes TAB, U+00A0 NO-BREAK SPACE, and U+3000
IDEOGRAPHIC SPACE. U+3000 is therefore Other despite also appearing in the CJK Symbols and
Punctuation candidate range.

### Other

Every scalar value not classified above is Other. Examples include ASCII punctuation and symbols,
full-width Latin letters such as `Ａ`, full-width digits such as `１`, half-width Katakana, emoji,
and CJK-related blocks not listed under CJK. Other is never an eligible member of a spacing pair.

## Spacing rules

The only eligible pairs are CJK–Latin and CJK–Digit, in either direction. The corresponding
configuration settings are independent.

| Pair, in either order | Configuration setting |
| --- | --- |
| CJK — Latin | `spacing.alphabets` |
| CJK — Digit | `spacing.digits` |

All other pairs, including Latin–Digit and CJK–CJK, are ignored.

| Rule | Required behavior |
| --- | --- |
| `require` | Insert one U+0020 SPACE when the eligible pair is directly adjacent. |
| `prohibit` | Delete a nonempty contiguous run of U+0020 SPACE between an eligible pair. |
| `ignore` | Do not change spacing. |

An existing U+0020 interrupts direct adjacency. Therefore, `require` does not normalize spacing:
it leaves both `漢 A` and `漢   A` unchanged. `prohibit` removes all ASCII spaces in an eligible
run, but it does not remove or cross a tab, line ending, full-width space, no-break space, or any
other non-ASCII whitespace.

| Input | `require` | `prohibit` |
| --- | --- | --- |
| `漢A` | `漢 A` | unchanged |
| `A漢` | `A 漢` | unchanged |
| `漢 1` | unchanged | `漢1` |
| `漢   A` | unchanged | `漢A` |
| `漢\tA` | unchanged | unchanged |
| `漢Ａ` | unchanged | unchanged |
| `漢。A` | unchanged | unchanged |

## Markdown prose

For Markdown documents, spacing rules apply to visible prose. This includes ordinary text,
emphasis, strikethrough, link text, and image descriptions.

Spacing rules do not modify or diagnose the following non-prose content:

- inline code and fenced code blocks;
- link destinations, titles, and reference labels;
- autolink URLs and email addresses;
- HTML tags;
- entity and numeric character references;
- backslash-escaped syntax; and
- malformed or otherwise unsafe inline constructs.

When Markdown cannot be interpreted safely, cjkfmt must preserve the affected construct rather
than partially formatting or diagnosing its contents.

## Checking and formatting

For the same Markdown input and configuration, `check` and `format` must select the same prose
content and apply the same spacing rules.

- `check` must report a spacing diagnostic exactly where `format` would make a spacing change.
- `format` must make every spacing change reported by `check` when run with the same configuration.
- Neither command may act on content excluded by the Markdown prose rules above.

This is the intended contract. The current checker does not yet share all of the formatter's
Markdown prose selection rules; the conformance gap is tracked in [issue #91].

[issue #91]: https://github.com/sgryjp/cjkfmt/issues/91

## Non-goals

This specification does not require cjkfmt to:

- normalize arbitrary whitespace or convert one whitespace character to another;
- insert or remove spaces for Latin–Digit, CJK–CJK, or other ineligible pairs;
- treat all Unicode decimal digits as Digit; or
- apply Markdown prose rules to non-Markdown documents.
