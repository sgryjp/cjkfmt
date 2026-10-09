---
name: Bug report
about: Create a report to help us improve
title: ""
labels: "bug, needs-triage"
assignees: ""
---

<!--
Before reporting, please search existing issues for related keywords:
https://github.com/sgryjp/cjkfmt/issues?q=is%3Aissue

Remove confidential information from examples, configuration, and logs,
but preserve the characters, spaces, and line breaks needed to reproduce the bug.
-->

## Bug description

<!-- Describe what is going wrong and how it affects your work. -->

## Minimal input

<!--
If the bug involves processing text, provide the smallest input that reproduces it.
Paste the original text in a fenced code block rather than a screenshot.
Preserve spaces and line breaks; do not paste only the rendered Markdown.
If trailing spaces or line endings matter, also attach the original file.

If you cannot provide an input example, explain why.
-->

## Command and reproduction steps

<!--
Paste the exact command in a fenced code block, including all options.
State whether the input comes from a file or stdin.
For file input, include the filename or its extension.
For stdin, make clear whether you used --language.

Include any other steps needed to reproduce the problem.
If reproduction is unreliable, describe what you tried and how often it occurs.
-->

<!--
If possible, check whether the bug also occurs in the latest release.
If you tested the main branch, include the commit ID.
You can report the bug even if you have not performed these checks.
-->

## Actual result

<!--
Paste the actual output and any error messages in fenced code blocks.
Include the exit code if it is relevant.
For formatting problems, include the resulting text, not just a description.
-->

## Expected result

<!--
Describe what you expected to happen.
For formatting problems, include the expected text in a fenced code block.
-->

## Configuration

<!--
Include any relevant .cjkfmt.json settings and CJKFMT_ environment variables.
For configuration files, state their location relative to the working directory,
or whether they are in the user's configuration directory.
If you are not using a configuration file or CJKFMT_ variables, say so.
-->

## Environment

<!-- Paste the output of cjkfmt --version rather than guessing the version. -->

- OS and version:
- cjkfmt version:
- Commit ID (if tested on main):

## Additional context (optional)

<!--
Add any other useful information, such as whether an earlier version worked.
Screenshots can help explain display problems, but should supplement the original text.
-->
