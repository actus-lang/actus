use std::io::{self, Write};

use serde_json::{Value, json};

use super::cancellation::CancellationToken;
use super::completion::{Context as CompletionContext, items};
use super::definition::find_definition;
use super::documents::DocumentStore;
use super::formatting::format_document;
use super::formatting::format_range;
use super::hover::find_hover;
use super::protocol::{
    DefinitionParams, FormattingParams, RangeFormattingParams, ResponseMetadata, TextEdit,
};
use super::query_bounds::{
    MAX_COMPLETION_ITEMS, MAX_HOVER_BYTES, MAX_SEMANTIC_TOKEN_VALUES, truncate_array,
    truncate_array_preserving_edges, truncate_text,
};
use super::query_cache::ParseSnapshot;
use super::semantic_tokens::full as semantic_tokens;
use crate::target::TargetSpec;

pub(super) fn completion(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let uri =
        params.get("textDocument").and_then(|document| document.get("uri")).and_then(Value::as_str);
    let document = uri.and_then(|value| store.get(value));
    let source = document.map(|value| value.text.as_str());
    let document_version = document.map(|value| value.version);
    let overlays = store.source_overlays();
    if source.is_none() {
        metadata.result_state = "unsupported";
    } else if requested_version(&params).is_some_and(|version| {
        store.get(uri.unwrap_or_default()).is_some_and(|document| document.version != version)
    }) {
        metadata.result_state = "stale";
        return super::server::respond(
            output,
            id,
            Value::Array(Vec::new()),
            metadata,
            cancellation,
        );
    }
    let position =
        params.get("position").and_then(|value| serde_json::from_value(value.clone()).ok());
    let (result, partial) = truncate_array_preserving_edges(
        items(
            uri.unwrap_or_default(),
            source,
            &overlays,
            position.as_ref(),
            CompletionContext { target, document_version, store, cancellation },
        ),
        MAX_COMPLETION_ITEMS,
    );
    if partial {
        metadata.result_state = "partial";
    }
    if source.is_some_and(|_| {
        matches!(store.parse_snapshot(uri.unwrap_or_default()), Some(ParseSnapshot::Invalid) | None)
    }) {
        metadata.result_state = "partial";
    }
    super::server::respond(output, id, result, metadata, cancellation)
}

pub(super) fn resolve_completion(
    id: Option<Value>,
    mut params: Value,
    store: &DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    if resolve_completion_item(&mut params, store, target) {
        metadata.result_state = "stale";
    }
    super::server::respond(output, id, params, metadata, cancellation)
}

pub(super) fn resolve_completion_item(
    item: &mut Value,
    store: &DocumentStore,
    target: &TargetSpec,
) -> bool {
    super::completion::resolve_item(item, store, target)
}

pub(super) fn semantic_tokens_full(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let params = serde_json::from_value::<FormattingParams>(params)
        .map_err(super::server::invalid_params)?;
    let Some(document) = store.get(&params.text_document.uri) else {
        metadata.result_state = "unsupported";
        return super::server::respond(
            output,
            id,
            serde_json::json!({"data": []}),
            metadata,
            cancellation,
        );
    };
    if stale_version(document.version, params.text_document.version) {
        metadata.result_state = "stale";
        return super::server::respond(
            output,
            id,
            serde_json::json!({"data": []}),
            metadata,
            cancellation,
        );
    }
    let mut result = semantic_tokens(&document.text, target);
    if let Some(data) = result.get_mut("data") {
        let (bounded, partial) = truncate_array(data.take(), MAX_SEMANTIC_TOKEN_VALUES);
        *data = bounded;
        if partial {
            metadata.result_state = "partial";
        }
    }
    super::server::respond(output, id, result, metadata, cancellation)
}

pub(super) fn definition(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let params = serde_json::from_value::<DefinitionParams>(params)
        .map_err(super::server::invalid_params)?;
    let Some(document) = store.get(&params.text_document.uri) else {
        metadata.result_state = "unsupported";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    };
    if stale_version(document.version, params.text_document.version) {
        metadata.result_state = "stale";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    }
    let result = find_definition(
        &params.text_document.uri,
        &document.text,
        &params.position,
        &store.source_overlays(),
    )
    .map(|location| {
        let mut value = json!({ "uri": location.uri, "range": location.range });
        if let Some(documentation) = location.documentation {
            value["documentation"] = Value::String(documentation);
        }
        value
    })
    .unwrap_or(Value::Null);
    super::server::respond(output, id, result, metadata, cancellation)
}

pub(super) fn hover(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let params = serde_json::from_value::<DefinitionParams>(params)
        .map_err(super::server::invalid_params)?;
    let Some(document) = store.get(&params.text_document.uri) else {
        metadata.result_state = "unsupported";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    };
    if stale_version(document.version, params.text_document.version) {
        metadata.result_state = "stale";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    }
    let result = find_hover(
        &params.text_document.uri,
        &document.text,
        &params.position,
        &store.source_overlays(),
        target,
    )
    .map(|info| {
        let (contents, partial) = truncate_text(&info.contents, MAX_HOVER_BYTES);
        if partial {
            metadata.result_state = "partial";
        }
        json!({ "contents": { "kind": "markdown", "value": contents }, "range": info.range })
    })
    .unwrap_or(Value::Null);
    super::server::respond(output, id, result, metadata, cancellation)
}

pub(super) fn formatting(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let params = serde_json::from_value::<FormattingParams>(params)
        .map_err(super::server::invalid_params)?;
    let Some(document) = store.get(&params.text_document.uri) else {
        metadata.result_state = "unsupported";
        return super::server::respond(output, id, serde_json::json!([]), metadata, cancellation);
    };
    if stale_version(document.version, params.text_document.version) {
        metadata.result_state = "stale";
        return super::server::respond(output, id, serde_json::json!([]), metadata, cancellation);
    }
    let source_is_valid =
        matches!(store.parse_snapshot(&params.text_document.uri), Some(ParseSnapshot::Valid(_)));
    let edits = format_document(&document.text)
        .map(|(range, new_text)| vec![TextEdit { range, new_text }])
        .unwrap_or_default();
    if !source_is_valid {
        metadata.result_state = "invalid";
    }
    super::server::respond(
        output,
        id,
        serde_json::to_value(edits).map_err(super::server::invalid_params)?,
        metadata,
        cancellation,
    )
}

pub(super) fn range_formatting(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let params = serde_json::from_value::<RangeFormattingParams>(params)
        .map_err(super::server::invalid_params)?;
    let Some(document) = store.get(&params.text_document.uri) else {
        metadata.result_state = "unsupported";
        return super::server::respond(output, id, json!([]), metadata, cancellation);
    };
    if stale_version(document.version, params.text_document.version) {
        metadata.result_state = "stale";
        return super::server::respond(output, id, json!([]), metadata, cancellation);
    }
    let source_is_valid =
        matches!(store.parse_snapshot(&params.text_document.uri), Some(ParseSnapshot::Valid(_)));
    let edits = format_range(&document.text, &params.range)
        .map(|(range, new_text)| vec![TextEdit { range, new_text }])
        .unwrap_or_default();
    if !source_is_valid {
        metadata.result_state = "invalid";
    }
    super::server::respond(
        output,
        id,
        serde_json::to_value(edits).map_err(super::server::invalid_params)?,
        metadata,
        cancellation,
    )
}

fn requested_version(params: &Value) -> Option<i64> {
    params.get("textDocument").and_then(|document| document.get("version")).and_then(Value::as_i64)
}

pub(super) fn stale_version(current: i64, requested: Option<i64>) -> bool {
    requested.is_some_and(|version| version != current)
}
