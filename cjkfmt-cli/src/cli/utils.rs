use cjkfmt_core::diagnostic::Diagnostic;
use yansi::Paint;

/// Returns a printable string representation of the diagnostic.
pub fn format_diagnostic(diagnostic: &Diagnostic) -> String {
    let filename = diagnostic.filename.as_deref().unwrap_or("<stdin>");
    let line = diagnostic.start.line;
    let column = diagnostic.start.column;

    let colon = ":".cyan();
    let filename = filename.white().bold();
    let line = (line + 1).to_string();
    let column = (column + 1).to_string();
    let code = diagnostic.code.yellow();
    let message = &diagnostic.message;

    format!(
        "{filename}{}{line}{}{column}{} {code} {message}",
        &colon, &colon, &colon
    )
}
