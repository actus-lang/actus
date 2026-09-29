use std::io::{self, Write};

use serde_json::Value;

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::protocol::ResponseMetadata;
use super::query_handlers::stale_version;
use crate::target::TargetSpec;

pub(super) fn dispatch(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let Some(uri) =
        params.get("textDocument").and_then(|document| document.get("uri")).and_then(Value::as_str)
    else {
        metadata.result_state = "unsupported";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    };
    let Some(document) = store.get(uri) else {
        metadata.result_state = "unsupported";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    };
    let requested = params
        .get("textDocument")
        .and_then(|document| document.get("version"))
        .and_then(Value::as_i64);
    if stale_version(document.version, requested) {
        metadata.result_state = "stale";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    }
    let result = super::semantic_model::query(uri, &document.text, store, target);
    super::server::respond(output, id, result, metadata, cancellation)
}
