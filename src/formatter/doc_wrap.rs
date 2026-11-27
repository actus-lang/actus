//! Wraps `"""` documentation strings whose lines pass the target width.
//!
//! Both emission paths of the formatter (the comment-preserving one and the
//! AST one) hand their documentation text to [`wrap_documentation`], so the
//! rule lives here only. Lines are never joined: a line that fits stays as it
//! is, and a line that does not is split at word boundaries.

use super::LINE_WIDTH;

/// Returns `text`, a complete `"""` documentation string as it will be
/// written (its first line without the declaration indentation `indent`, the
/// following lines already indented), with every over-long line wrapped.
///
/// - A single-line documentation string that fits is unchanged; one that does
///   not becomes the block form at `indent`, wrapped at word boundaries.
/// - In a multi-line documentation string only the lines whose output width
///   exceeds [`LINE_WIDTH`] change. A continuation line keeps the leading
///   whitespace of the line it continues, plus 4 columns for an indented
///   `name: text` item and plus 2 columns for a `- text` list item.
/// - Lines between fences that start with three backticks are never changed.
/// - A word wider than the line stays whole on a line of its own.
pub(super) fn wrap_documentation(text: &str, indent: &str) -> String {
    let indent_width = indent.chars().count();
    if !text.contains('\n') {
        return wrap_single_line(text, indent, indent_width);
    }
    let mut lines = Vec::new();
    let mut fenced = false;
    for (index, line) in text.split('\n').enumerate() {
        let is_fence = line.trim_start().starts_with("```");
        if is_fence {
            fenced = !fenced;
        }
        // Only the first line is written after the declaration indentation.
        let offset = if index == 0 { indent_width } else { 0 };
        if fenced || is_fence || offset + line.trim_end_matches('\r').chars().count() <= LINE_WIDTH
        {
            lines.push(line.to_owned());
        } else {
            lines.extend(wrap_line(line, (index == 0).then_some(indent), offset));
        }
    }
    lines.join("\n")
}

/// A one-line `"""text"""` that does not fit becomes the block form.
fn wrap_single_line(text: &str, indent: &str, indent_width: usize) -> String {
    let inner = text.strip_prefix("\"\"\"").and_then(|rest| rest.strip_suffix("\"\"\""));
    let Some(inner) = inner.filter(|inner| !inner.trim().is_empty()) else {
        return text.to_owned();
    };
    if indent_width + text.chars().count() <= LINE_WIDTH {
        return text.to_owned();
    }
    let lines = wrap_words(inner.split_whitespace(), indent, indent, 0, 1);
    format!("\"\"\"\n{}\n{indent}\"\"\"", lines.join("\n"))
}

/// Splits one over-long line of a multi-line documentation string.
/// `opening_indent` is `Some` for the line that carries the opening `"""`.
fn wrap_line(line: &str, opening_indent: Option<&str>, offset: usize) -> Vec<String> {
    let body = line.trim_start();
    let leading = &line[..line.len() - body.len()];
    let (continuation, glued_words) = match opening_indent {
        Some(indent) => (indent.to_owned(), 1),
        None => continuation_of(leading, body),
    };
    wrap_words(body.split_whitespace(), leading, &continuation, offset, glued_words)
}

/// The indentation continuing a line, and how many of its leading words must
/// stay together (a list marker or `name:` never ends a line alone).
fn continuation_of(leading: &str, body: &str) -> (String, usize) {
    if leading.is_empty() {
        return (String::new(), 1);
    }
    if body.starts_with("- ") {
        return (format!("{leading}  "), 2);
    }
    let is_item = body.split_once(':').is_some_and(|(name, rest)| {
        !name.is_empty()
            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            && rest.starts_with(char::is_whitespace)
    });
    if is_item { (format!("{leading}    "), 2) } else { (leading.to_owned(), 1) }
}

/// Greedily packs `words` into lines no wider than [`LINE_WIDTH`]. The first
/// line starts with `first_prefix` after `offset` columns that are already
/// written, the others with `continuation`. The first `glued_words` words
/// share a line even when it is too wide.
fn wrap_words<'a>(
    words: impl Iterator<Item = &'a str>,
    first_prefix: &str,
    continuation: &str,
    offset: usize,
    glued_words: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = first_prefix.to_owned();
    let mut width = offset + first_prefix.chars().count();
    let (mut count, mut required) = (0, glued_words);
    for word in words {
        let word_width = word.chars().count();
        if count >= required && width + 1 + word_width > LINE_WIDTH {
            lines.push(std::mem::replace(&mut current, continuation.to_owned()));
            width = continuation.chars().count();
            (count, required) = (0, 1);
        }
        if count > 0 {
            current.push(' ');
            width += 1;
        }
        current.push_str(word);
        width += word_width;
        count += 1;
    }
    lines.push(current);
    lines
}
