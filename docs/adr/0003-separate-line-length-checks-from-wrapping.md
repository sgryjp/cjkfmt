# Keep line-length checks independent of wrapping

Status: accepted

Formatting and checking answer different questions: formatting may split a line at a safe point,
while W001 reports overflow on the original physical input regardless of whether a break is
available. Add a `preserve` wrapping policy for users who want spacing edits without line-break
changes, while keeping `split` as the default so existing configurations retain their behavior.
Reflow and a setting to disable W001 are deferred; W001 remains independent of wrapping so its
coverage does not depend on formatting policy.
