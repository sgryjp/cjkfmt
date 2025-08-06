use cjkfmt_core::diagnostic::Diagnostic;
use yansi::Paint;

/// Returns a printable string representation of the diagnostic.
pub fn format_diagnostic(diagnostic: &Diagnostic) -> String {
    let filename = diagnostic.filename.as_deref().unwrap_or("<stdin>");
    let line = diagnostic.start.line;
    let column = diagnostic.start.column;
    let buf = format!(
        "{}\0{}\0{}\0{}\0{}",
        filename,
        line + 1,
        column + 1,
        diagnostic.code,
        diagnostic.message
    );
    let tokens: Vec<&str> = buf.split('\0').collect();
    let colon = ":".cyan();
    let filename = tokens[0].white().bold();
    let line = tokens[1];
    let column = tokens[2];
    let code = tokens[3].yellow();
    let message = tokens[4];

    format!(
        "{filename}{}{line}{}{column}{} {code} {message}",
        &colon, &colon, &colon
    )
}
