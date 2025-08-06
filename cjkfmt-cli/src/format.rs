use cjkfmt_core::lines_inclusive::LinesInclusiveExt;

use crate::{
    config::Config,
    line_break::{BreakPoint, LineBreaker},
};

pub(crate) fn format_one_file<W: std::io::Write>(
    stdout: &mut W,
    config: &Config,
    content: &str,
) -> Result<(), anyhow::Error> {
    let line_breaker = LineBreaker::builder()
        .ambiguous_width(config.ambiguous_width)
        .max_width(config.max_width)
        .build()?;

    // Iterate over each line in the input content, including line endings
    for line in content.lines_inclusive() {
        let mut remainings = line;

        // Iterate over wrap points in the line
        while let BreakPoint::WrapPoint {
            overflow_pos,
            adjustment,
        } = line_breaker.next_line_break(remainings)
        {
            // Write the part before the wrap point
            let (before, after) = remainings.split_at(overflow_pos - adjustment);
            writeln!(stdout, "{before}")?;
            remainings = after;
        }

        // Write any remaining part of the line after the last wrap point
        write!(stdout, "{remainings}")?;
    }
    Ok(())
}
