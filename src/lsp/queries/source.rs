use std::io::{self, Write};

use serde_json::{Value, json};

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::formatting::{format_document, organize_imports};
use super::protocol::ResponseMetadata;
use super::query_handlers::stale_version;
use crate::target::TargetSpec;

pub(super) struct Context<'a, W> {
    pub(super) store: &'a DocumentStore,
    pub(super) target: &'a TargetSpec,
    pub(super) output: &'a mut W,
    pub(super) metadata: ResponseMetadata,
    pub(super) cancellation: Option<&'a CancellationToken>,
}

pub(super) fn dispatch<W: Write>(
    method: &str,
    id: Option<Value>,
    params: Value,
    mut context: Context<'_, W>,
) -> io::Result<()> {
    match method {
        "textDocument/codeAction" => code_action(id, params, &mut context),
        "textDocument/codeLens" => code_lens(id, params, &mut context),
        "actus/run" => run_workflow(id, params, &mut context),
        _ => Ok(()),
    }
}

fn code_action(
    id: Option<Value>,
    params: Value,
    context: &mut Context<'_, impl Write>,
) -> io::Result<()> {
    let store = context.store;
    let output = &mut *context.output;
    let mut metadata = context.metadata.clone();
    let cancellation = context.cancellation;
    let Some(uri) =
        params.get("textDocument").and_then(|document| document.get("uri")).and_then(Value::as_str)
    else {
        return super::server::respond(output, id, json!([]), metadata, cancellation);
    };
    let requested = params
        .get("textDocument")
        .and_then(|document| document.get("version"))
        .and_then(Value::as_i64);
    let Some(document) = store.get(uri) else {
        metadata.result_state = "unsupported";
        return super::server::respond(output, id, json!([]), metadata, cancellation);
    };
    if stale_version(document.version, requested) {
        metadata.result_state = "stale";
        return super::server::respond(output, id, json!([]), metadata, cancellation);
    }
    let Some((range, new_text)) = format_document(&document.text) else {
        metadata.result_state = "invalid";
        return super::server::respond(output, id, json!([]), metadata, cancellation);
    };
    let format_action = json!({"title":"Format document","kind":"source.format","edit":{"changes":{uri:[{"range":range,"newText":new_text}]}}});
    let mut actions = vec![format_action];
    if let Some((range, new_text)) = organize_imports(&document.text) {
        actions.push(json!({"title":"Organize imports and facade opens","kind":"source.organizeImports","edit":{"changes":{uri:[{"range":range,"newText":new_text}]}}}));
    }
    super::server::respond(output, id, Value::Array(actions), metadata, cancellation)
}

pub(super) fn rename(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
    prepare: bool,
) -> io::Result<()> {
    let stale = params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
        .and_then(|uri| {
            params
                .get("textDocument")
                .and_then(|document| document.get("version"))
                .and_then(Value::as_i64)
                .map(|version| (uri, version))
        })
        .is_some_and(|(uri, version)| {
            store.get(uri).is_some_and(|current| current.version != version)
        });
    if stale {
        metadata.result_state = "stale";
        let result = if prepare { Value::Null } else { json!({"changes": {}}) };
        return super::server::respond(output, id, result, metadata, cancellation);
    }
    let result = if prepare {
        super::rename::prepare(&params, store)
    } else {
        super::rename::rename(&params, store)
    };
    super::server::respond(output, id, result, metadata, cancellation)
}

fn code_lens(
    id: Option<Value>,
    params: Value,
    context: &mut Context<'_, impl Write>,
) -> io::Result<()> {
    let store = context.store;
    let target = context.target;
    let output = &mut *context.output;
    let metadata = context.metadata.clone();
    let cancellation = context.cancellation;
    let Some(uri) =
        params.get("textDocument").and_then(|value| value.get("uri")).and_then(Value::as_str)
    else {
        return empty_code_lens(output, id, metadata, cancellation, "available");
    };
    let Some(document) = store.get(uri) else {
        return empty_code_lens(output, id, metadata, cancellation, "unsupported");
    };
    if requested_version(&params).is_some_and(|version| version != document.version) {
        return empty_code_lens(output, id, metadata, cancellation, "stale");
    }
    let result =
        super::code_lens::entry_lenses(uri, &document.text, document.version, store, target);
    super::server::respond(output, id, result, metadata, cancellation)
}

fn run_workflow(
    id: Option<Value>,
    params: Value,
    context: &mut Context<'_, impl Write>,
) -> io::Result<()> {
    let store = context.store;
    let target = context.target;
    let output = &mut *context.output;
    let mut metadata = context.metadata.clone();
    let cancellation = context.cancellation;
    let result = super::workflow::run(&params, store, target, &mut metadata, cancellation);
    super::server::respond(output, id, result, metadata, cancellation)
}

fn empty_code_lens(
    output: &mut impl Write,
    id: Option<Value>,
    metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
    result_state: &'static str,
) -> io::Result<()> {
    let metadata = ResponseMetadata { result_state, ..metadata };
    super::server::respond(output, id, json!([]), metadata, cancellation)
}

fn requested_version(params: &Value) -> Option<i64> {
    params.get("textDocument").and_then(|document| document.get("version")).and_then(Value::as_i64)
}
