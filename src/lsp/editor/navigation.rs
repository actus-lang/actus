use std::io::{self, Write};

use serde_json::{Value, json};

use crate::ast::TopLevelDecl;
use crate::lexer::{SourceSpan, Token, TokenKind, scan};
use crate::parser::parse;

use super::cancellation::CancellationToken;
use super::documents::{Document, DocumentStore};
use super::position::{LineIndex, LspPosition, LspRange};
use super::protocol::ResponseMetadata;

const MAX_NAVIGATION_RESULTS: usize = 512;

pub(super) fn dispatch<W: Write>(
    method: &str,
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    output: &mut W,
    metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    if navigation_is_stale(&params, store) {
        let metadata = ResponseMetadata { result_state: "stale", ..metadata };
        super::server::respond(output, id, empty_result(method), metadata, cancellation)?;
        return Ok(());
    }
    let result = match method {
        "textDocument/declaration"
        | "textDocument/typeDefinition"
        | "textDocument/implementation" => definition_like(method, params, store),
        "textDocument/references" => references(params, store),
        "textDocument/documentSymbol" => document_symbols(params, store),
        "workspace/symbol" => workspace_symbols(params, store),
        "textDocument/prepareCallHierarchy" => prepare_call_hierarchy(params, store),
        "callHierarchy/incomingCalls" => incoming_calls(params, store),
        "callHierarchy/outgoingCalls" => outgoing_calls(params, store),
        _ => return Ok(()),
    };
    super::server::respond(output, id, result, metadata, cancellation)?;
    Ok(())
}

fn navigation_is_stale(params: &Value, store: &DocumentStore) -> bool {
    let Some(uri) = document_uri(params) else { return false };
    let Some(version) = params
        .get("textDocument")
        .and_then(|document| document.get("version"))
        .and_then(Value::as_i64)
    else {
        return false;
    };
    store.get(&uri).is_some_and(|document| document.version != version)
}

fn empty_result(_method: &str) -> Value {
    Value::Array(Vec::new())
}

fn definition_like(method: &str, params: Value, store: &DocumentStore) -> Value {
    let Some((uri, position)) = text_position(&params) else { return Value::Null };
    let Some(document) = store.get(&uri) else { return Value::Null };
    let overlays = store.source_overlays();
    let result = super::definition::find_definition(&uri, &document.text, &position, &overlays)
        .map(|location| {
            let mut value = json!({"uri": location.uri, "range": location.range});
            if let Some(documentation) = location.documentation {
                value["documentation"] = Value::String(documentation);
            }
            value
        });
    if method == "textDocument/implementation" {
        return result.map_or(Value::Array(Vec::new()), |location| Value::Array(vec![location]));
    }
    result.unwrap_or(Value::Null)
}

fn references(params: Value, store: &DocumentStore) -> Value {
    let Some((uri, position)) = text_position(&params) else { return Value::Array(Vec::new()) };
    let Some(document) = store.get(&uri) else { return Value::Array(Vec::new()) };
    let Some(target) = super::definition::find_definition(
        &uri,
        &document.text,
        &position,
        &store.source_overlays(),
    ) else {
        return Value::Array(Vec::new());
    };
    let mut locations = Vec::new();
    for (document_uri, source) in store.document_sources() {
        for token in identifier_tokens(&source) {
            let TokenKind::Identifier(_) = token.kind else { continue };
            let token_position = LineIndex::new(&source).position(&source, token.span.start);
            let Some(definition) = super::definition::find_definition(
                &document_uri,
                &source,
                &token_position,
                &store.source_overlays(),
            ) else {
                continue;
            };
            if definition.uri != target.uri || definition.range != target.range {
                continue;
            }
            locations.push(json!({"uri": document_uri, "range": range(&source, token.span)}));
            if locations.len() == MAX_NAVIGATION_RESULTS {
                return Value::Array(locations);
            }
        }
    }
    Value::Array(locations)
}

fn document_symbols(params: Value, store: &DocumentStore) -> Value {
    let Some(uri) = document_uri(&params) else { return Value::Array(Vec::new()) };
    let Some(document) = store.get(&uri) else { return Value::Array(Vec::new()) };
    Value::Array(symbols_for_source(&uri, &document.text))
}

fn workspace_symbols(params: Value, store: &DocumentStore) -> Value {
    let query = params.get("query").and_then(Value::as_str).unwrap_or_default();
    let mut symbols = store
        .document_sources()
        .into_iter()
        .flat_map(|(uri, source)| symbols_for_source(&uri, &source))
        .filter(|symbol| {
            symbol.get("name").and_then(Value::as_str).is_some_and(|name| {
                query.is_empty() || name.to_ascii_lowercase().contains(&query.to_ascii_lowercase())
            })
        })
        .collect::<Vec<_>>();
    symbols.sort_by_key(|symbol| {
        (
            symbol.get("name").and_then(Value::as_str).unwrap_or_default().to_owned(),
            symbol.get("uri").and_then(Value::as_str).unwrap_or_default().to_owned(),
        )
    });
    symbols.truncate(MAX_NAVIGATION_RESULTS);
    Value::Array(symbols)
}

fn prepare_call_hierarchy(params: Value, store: &DocumentStore) -> Value {
    let Some((uri, position)) = text_position(&params) else { return Value::Array(Vec::new()) };
    let Some(document) = store.get(&uri) else { return Value::Array(Vec::new()) };
    let Some(name) = identifier_at(&document.text, &position) else {
        return Value::Array(Vec::new());
    };
    let Some(symbol) = callable_symbol(&uri, &document.text, &name, &position) else {
        return Value::Array(Vec::new());
    };
    Value::Array(vec![symbol])
}

fn incoming_calls(params: Value, store: &DocumentStore) -> Value {
    let Some(item) = params.get("item") else { return Value::Array(Vec::new()) };
    let Some(name) = item.get("name").and_then(Value::as_str) else {
        return Value::Array(Vec::new());
    };
    let mut calls = Vec::new();
    for (uri, source) in store.document_sources() {
        for call_span in call_spans(&source, name) {
            let Some(caller) = enclosing_callable(&uri, &source, call_span.start) else { continue };
            calls.push(json!({"from": caller, "fromRanges": [range(&source, call_span)]}));
            if calls.len() == MAX_NAVIGATION_RESULTS {
                return Value::Array(calls);
            }
        }
    }
    Value::Array(calls)
}

fn outgoing_calls(params: Value, store: &DocumentStore) -> Value {
    let Some(item) = params.get("item") else { return Value::Array(Vec::new()) };
    let Some(uri) = item.get("uri").and_then(Value::as_str) else {
        return Value::Array(Vec::new());
    };
    let Some(source) = store.get(uri).map(|document| document.text.as_str()) else {
        return Value::Array(Vec::new());
    };
    let Some(name) = item.get("name").and_then(Value::as_str) else {
        return Value::Array(Vec::new());
    };
    let Some(caller_span) = declaration_span(source, name) else { return Value::Array(Vec::new()) };
    let calls = call_spans(source.get(caller_span.start..caller_span.end).unwrap_or_default(), "");
    let calls = calls.into_iter().filter_map(|span| {
        let absolute = SourceSpan::new(caller_span.start + span.start, caller_span.start + span.end);
        let token_name = identifier_at_offset(source, absolute.start)?;
        if token_name == name { return None }
        let target = super::definition::find_definition(
            uri,
            source,
            &LineIndex::new(source).position(source, absolute.start),
            &store.source_overlays(),
        )?;
        Some(json!({"to": {"name": token_name, "kind": 12, "uri": target.uri, "range": target.range, "selectionRange": target.range}, "fromRanges": [range(source, absolute)]}))
    }).take(MAX_NAVIGATION_RESULTS).collect::<Vec<_>>();
    Value::Array(calls)
}

fn symbols_for_source(uri: &str, source: &str) -> Vec<Value> {
    let Ok(program) = parse(scan(source).0) else { return Vec::new() };
    program.declarations.iter().filter_map(|declaration| {
        let (name, span, kind) = declaration_identity(declaration)?;
        let selection = identifier_span(source, span, name)?;
        Some(json!({"name": name, "kind": kind, "uri": uri, "range": range(source, span), "selectionRange": range(source, selection)}))
    }).take(MAX_NAVIGATION_RESULTS).collect()
}

fn declaration_identity(declaration: &TopLevelDecl) -> Option<(&str, SourceSpan, u8)> {
    match declaration {
        TopLevelDecl::Verb(value) => Some((&value.name, value.span, 12)),
        TopLevelDecl::ExternalVerb(value) => Some((&value.name, value.span, 12)),
        TopLevelDecl::Struct(value) => Some((&value.name, value.span, 23)),
        TopLevelDecl::Pack(value) => Some((&value.name, value.span, 23)),
        TopLevelDecl::Enum(value) => Some((&value.name, value.span, 5)),
        TopLevelDecl::Role(value) => Some((&value.name, value.span, 11)),
        _ => None,
    }
}

fn callable_symbol(uri: &str, source: &str, name: &str, position: &LspPosition) -> Option<Value> {
    let span = declaration_span(source, name)?;
    let selection = identifier_span(source, span, name)?;
    let index = LineIndex::new(source);
    let _ = position;
    Some(
        json!({"name": name, "kind": 12, "uri": uri, "range": range(source, span), "selectionRange": range(source, selection), "detail": "Actus verb" , "_span": {"start": index.position(source, span.start)}}),
    )
}

fn enclosing_callable(uri: &str, source: &str, offset: usize) -> Option<Value> {
    let program = parse(scan(source).0).ok()?;
    program.declarations.iter().find_map(|declaration| {
        let TopLevelDecl::Verb(verb) = declaration else { return None };
        if !(verb.span.start..=verb.span.end).contains(&offset) { return None }
        let selection = identifier_span(source, verb.span, &verb.name)?;
        Some(json!({"name": verb.name, "kind": 12, "uri": uri, "range": range(source, verb.span), "selectionRange": range(source, selection)}))
    })
}

fn declaration_span(source: &str, name: &str) -> Option<SourceSpan> {
    let program = parse(scan(source).0).ok()?;
    program.declarations.iter().find_map(|declaration| {
        let (declared, span, _) = declaration_identity(declaration)?;
        (declared == name).then_some(span)
    })
}

fn text_position(params: &Value) -> Option<(String, LspPosition)> {
    let uri = document_uri(params)?;
    let position = serde_json::from_value(params.get("position")?.clone()).ok()?;
    Some((uri, position))
}

fn document_uri(params: &Value) -> Option<String> {
    params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn identifier_at(source: &str, position: &LspPosition) -> Option<String> {
    let offset = LineIndex::new(source).byte_offset(source, position)?;
    identifier_at_offset(source, offset)
}

fn identifier_at_offset(source: &str, offset: usize) -> Option<String> {
    scan(source).0.into_iter().find_map(|token| {
        (token.span.start <= offset && offset <= token.span.end).then_some(match token.kind {
            TokenKind::Identifier(name) => Some(name),
            _ => None,
        })?
    })
}

fn identifier_tokens(source: &str) -> Vec<Token> {
    scan(source)
        .0
        .into_iter()
        .filter(|token| matches!(token.kind, TokenKind::Identifier(_)))
        .take(MAX_NAVIGATION_RESULTS)
        .collect()
}

fn call_spans(source: &str, name: &str) -> Vec<SourceSpan> {
    let tokens = scan(source).0;
    tokens
        .windows(2)
        .filter_map(|window| {
            let Token { kind: TokenKind::Identifier(value), span } = &window[0] else {
                return None;
            };
            if (!name.is_empty() && value != name)
                || !matches!(window[1].kind, TokenKind::LeftParen)
            {
                return None;
            }
            Some(*span)
        })
        .take(MAX_NAVIGATION_RESULTS)
        .collect()
}

fn identifier_span(source: &str, span: SourceSpan, name: &str) -> Option<SourceSpan> {
    scan(source.get(span.start..span.end)?).0.into_iter().find_map(|token| match token.kind {
        TokenKind::Identifier(value) if value == name => {
            Some(SourceSpan::new(span.start + token.span.start, span.start + token.span.end))
        }
        _ => None,
    })
}

fn range(source: &str, span: SourceSpan) -> LspRange {
    let index = LineIndex::new(source);
    LspRange { start: index.position(source, span.start), end: index.position(source, span.end) }
}

trait DocumentSources {
    fn document_sources(&self) -> Vec<(String, String)>;
}

impl DocumentSources for DocumentStore {
    fn document_sources(&self) -> Vec<(String, String)> {
        self.documents_snapshot()
            .into_iter()
            .map(|(uri, Document { text, .. })| (uri, text))
            .collect()
    }
}
