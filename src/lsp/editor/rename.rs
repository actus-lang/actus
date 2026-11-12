use serde_json::{Value, json};

use crate::lexer::{TokenKind, scan};

use super::definition::{DefinitionLocation, find_definition};
use super::documents::DocumentStore;
use super::position::{LineIndex, LspPosition, LspRange};

const MAX_RENAME_EDITS: usize = 1024;

pub(super) fn prepare(params: &Value, store: &DocumentStore) -> Value {
    let Some((uri, source, position)) = request_context(params, store) else {
        return Value::Null;
    };
    let Some(name) = identifier_at(source, &position) else { return Value::Null };
    let Some(definition) = find_definition(&uri, source, &position, &store.source_overlays())
    else {
        return Value::Null;
    };
    let _ = definition;
    json!({"range": identifier_range(source, &name, &position), "placeholder": name})
}

pub(super) fn rename(params: &Value, store: &DocumentStore) -> Value {
    let Some(new_name) = params.get("newName").and_then(Value::as_str) else {
        return Value::Null;
    };
    if !valid_identifier(new_name) {
        return Value::Null;
    }
    let Some((uri, source, position)) = request_context(params, store) else {
        return Value::Null;
    };
    let Some(target) = find_definition(&uri, source, &position, &store.source_overlays()) else {
        return Value::Null;
    };
    if !is_open_target(&target, store) {
        return Value::Null;
    }
    let overlays = store.source_overlays();
    let mut changes = serde_json::Map::new();
    let mut edit_count = 0;
    for (document_uri, document) in store.documents_snapshot() {
        let edits = rename_edits(
            &document_uri,
            &document.text,
            &target,
            &overlays,
            new_name,
            &mut edit_count,
        );
        if !edits.is_empty() {
            changes.insert(document_uri, Value::Array(edits));
        }
        if edit_count >= MAX_RENAME_EDITS {
            break;
        }
    }
    if edit_count > 0 { json!({"changes": changes}) } else { Value::Null }
}

fn request_context<'a>(
    params: &Value,
    store: &'a DocumentStore,
) -> Option<(String, &'a str, LspPosition)> {
    let document = params.get("textDocument")?;
    let uri = document.get("uri")?.as_str()?.to_owned();
    let version = document.get("version").and_then(Value::as_i64);
    let source_document = store.get(&uri)?;
    if version.is_some_and(|value| value != source_document.version) {
        return None;
    }
    let position = serde_json::from_value(params.get("position")?.clone()).ok()?;
    Some((uri, source_document.text.as_str(), position))
}

fn rename_edits(
    uri: &str,
    source: &str,
    target: &DefinitionLocation,
    overlays: &std::collections::HashMap<std::path::PathBuf, String>,
    new_name: &str,
    edit_count: &mut usize,
) -> Vec<Value> {
    let mut edits = Vec::new();
    let tokens = scan(source).0;
    for token in tokens {
        if *edit_count >= MAX_RENAME_EDITS {
            break;
        }
        let TokenKind::Identifier(_) = token.kind else { continue };
        let candidate_range = range(source, token.span.start, token.span.end);
        if uri == target.uri && candidate_range == target.range {
            edits.push(json!({"range": candidate_range, "newText": new_name}));
            *edit_count += 1;
            continue;
        }
        let position = LineIndex::new(source).position(source, token.span.start);
        let Some(definition) = find_definition(uri, source, &position, overlays) else {
            continue;
        };
        if definition.uri != target.uri || definition.range != target.range {
            continue;
        }
        edits.push(json!({
            "range": candidate_range,
            "newText": new_name,
        }));
        *edit_count += 1;
    }
    edits
}

fn identifier_at(source: &str, position: &LspPosition) -> Option<String> {
    let offset = LineIndex::new(source).byte_offset(source, position)?;
    scan(source).0.into_iter().find_map(|token| {
        (token.span.start <= offset && offset <= token.span.end).then_some(match token.kind {
            TokenKind::Identifier(name) => Some(name),
            _ => None,
        })?
    })
}

fn identifier_range(source: &str, name: &str, position: &LspPosition) -> LspRange {
    let offset = LineIndex::new(source).byte_offset(source, position).unwrap_or_default();
    scan(source)
        .0
        .into_iter()
        .find_map(|token| {
            matches!(token.kind, TokenKind::Identifier(ref value) if value == name
                && token.span.start <= offset && offset <= token.span.end)
            .then(|| range(source, token.span.start, token.span.end))
        })
        .unwrap_or_else(|| range(source, offset, offset))
}

fn valid_identifier(name: &str) -> bool {
    let tokens = scan(name)
        .0
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Eof))
        .collect::<Vec<_>>();
    tokens.len() == 1
        && matches!(tokens[0].kind, TokenKind::Identifier(_))
        && tokens[0].span.start == 0
        && tokens[0].span.end == name.len()
}

fn is_open_target(target: &DefinitionLocation, store: &DocumentStore) -> bool {
    store.get(&target.uri).is_some()
}

fn range(source: &str, start: usize, end: usize) -> LspRange {
    let index = LineIndex::new(source);
    LspRange { start: index.position(source, start), end: index.position(source, end) }
}
