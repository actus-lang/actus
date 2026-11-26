//! The repeat directive of catalog blocks (ADR-0123). A comment line that holds
//! `catalog: repeat N` starts a region and a comment line that holds
//! `catalog: end` ends it. The lines between them are written N times, and `{n}`
//! in them is the 1-based index, zero-padded to the width of N. The marker
//! lines are comments and are written as they are.

const REPEAT: &str = "catalog: repeat ";
const END: &str = "catalog: end";

/// The text of a comment line (`# ...`, the Actus and TOML line comment).
fn comment(line: &str) -> Option<&str> {
    line.trim_start().strip_prefix('#')
}

/// Writes the regions of `body` out. An unclosed or nested region, a count that
/// is zero or not a number, and an `end` without a region are errors.
pub fn expand(body: &str) -> Result<String, String> {
    let mut output = String::new();
    // The count and the lines of the open region.
    let mut region: Option<(usize, Vec<&str>)> = None;
    for line in body.lines() {
        let marker = comment(line);
        if let Some(count) = marker.and_then(|text| text.split_once(REPEAT)).map(|pair| pair.1) {
            if region.is_some() {
                return Err("`catalog: repeat` inside a region; regions do not nest".to_owned());
            }
            let count =
                count.trim().parse::<usize>().ok().filter(|count| *count > 0).ok_or_else(|| {
                    format!("`catalog: repeat {}` needs a number greater than zero", count.trim())
                })?;
            region = Some((count, Vec::new()));
        } else if marker.is_some_and(|text| text.contains(END)) {
            let (count, lines) =
                region.take().ok_or("`catalog: end` without `catalog: repeat`".to_owned())?;
            let width = count.to_string().len();
            for index in 1..=count {
                let number = format!("{index:0width$}");
                for inner in &lines {
                    output.push_str(&inner.replace("{n}", &number));
                    output.push('\n');
                }
            }
            output.push_str(line);
            output.push('\n');
            continue;
        } else if let Some((_, lines)) = region.as_mut() {
            lines.push(line);
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    match region {
        Some(_) => Err("`catalog: repeat` is never closed by `catalog: end`".to_owned()),
        None => Ok(output),
    }
}
