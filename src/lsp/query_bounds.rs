use serde_json::Value;

pub(super) const MAX_COMPLETION_ITEMS: usize = 512;
pub(super) const MAX_SEMANTIC_TOKEN_VALUES: usize = 100_000;
pub(super) const MAX_HOVER_BYTES: usize = 64 * 1024;
pub(super) const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub(super) fn truncate_array(mut result: Value, limit: usize) -> (Value, bool) {
    let Some(items) = result.as_array_mut() else { return (result, false) };
    if items.len() <= limit {
        return (result, false);
    }
    items.truncate(limit);
    (result, true)
}

pub(super) fn truncate_array_preserving_edges(mut result: Value, limit: usize) -> (Value, bool) {
    let Some(items) = result.as_array_mut() else { return (result, false) };
    if items.len() <= limit {
        return (result, false);
    }
    let head = limit / 2;
    let tail = limit - head;
    let tail_items = items.split_off(items.len() - tail);
    items.truncate(head);
    items.extend(tail_items);
    (result, true)
}

pub(super) fn truncate_text(text: &str, limit: usize) -> (String, bool) {
    if text.len() <= limit {
        return (text.to_owned(), false);
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_owned(), true)
}
