use std::io::{self, Write};
use std::sync::Arc;

use serde_json::Value;

use super::cancellation::{CancellationRegistry, CancellationToken};
use super::diagnostics::analyze_document_for_target;
use super::documents::DocumentStore;
use super::progress;
use super::protocol::{
    DidChangeParams, DidOpenParams, ErrorResponse, JsonRpcError, Notification,
    PublishDiagnosticsParams, Request, Response, ResponseMetadata,
};
use super::protocol_contract::{SessionState, response_metadata};
use super::request_dispatch::{dispatch_request, is_expensive};
use super::transport::{spawn_reader, write_message};
use crate::target::TargetSpec;

pub fn run_stdio() -> io::Result<()> {
    let mut output = io::stdout().lock();
    let cancellations = Arc::new(CancellationRegistry::default());
    let messages = spawn_reader(Arc::clone(&cancellations));
    let mut store = DocumentStore::default();
    let mut target = TargetSpec::host().expect("host target must be supported");
    let mut state = SessionState::default();
    while let Ok(message) = messages.recv() {
        let request = match serde_json::from_slice::<Request>(&message) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("actus lsp: invalid JSON-RPC message: {error}");
                continue;
            }
        };
        let request_id = request.id.clone();
        let request_params = request.params.clone();
        let cancellation = cancellations.token(request_id.as_ref());
        match handle_request(
            request,
            &mut store,
            &mut target,
            &mut state,
            &mut output,
            cancellation.as_ref(),
        ) {
            Ok(true) => break,
            Ok(false) => {}
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                if let Some(id) = request_id {
                    let metadata = response_metadata(&target, &request_params, &store);
                    write_error(&mut output, id, -32602, error.to_string(), metadata)?;
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn handle_request(
    request: Request,
    store: &mut DocumentStore,
    target: &mut TargetSpec,
    state: &mut SessionState,
    output: &mut impl Write,
    cancellation: Option<&CancellationToken>,
) -> io::Result<bool> {
    let metadata = response_metadata(target, &request.params, store);
    if canceled_request(&request, output, metadata.clone(), cancellation)? {
        return Ok(false);
    }
    if *state == SessionState::ShuttingDown && request.method != "exit" {
        return write_lifecycle_error(output, request.id, "server is shutting down", metadata);
    }
    let progress_token = if is_expensive(&request.method) {
        request.id.as_ref().map(|id| progress::begin(output, id)).transpose()?
    } else {
        None
    };
    if let Some(token) = progress_token.as_deref() {
        progress::report(output, token, "compiler-backed query")?;
    }
    let result = dispatch_request(request, store, target, state, output, cancellation);
    if let Some(token) = progress_token {
        progress::finish(output, &token, cancellation.is_some_and(CancellationToken::is_canceled))?;
    }
    result
}

fn canceled_request(
    request: &Request,
    output: &mut impl Write,
    metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<bool> {
    if !cancellation.is_some_and(CancellationToken::is_canceled) {
        return Ok(false);
    }
    if let Some(id) = request.id.clone() {
        write_error(output, id, -32800, "request canceled".to_owned(), metadata)?;
    }
    Ok(true)
}

pub(super) fn open_document(
    params: Value,
    store: &mut DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
) -> io::Result<()> {
    let params = serde_json::from_value::<DidOpenParams>(params).map_err(invalid_params)?;
    if let Err(error) = store.open(
        params.text_document.uri.clone(),
        params.text_document.version,
        params.text_document.text,
    ) {
        eprintln!("actus lsp: {error}");
        return Ok(());
    }
    publish(output, &params.text_document.uri, store, target)
}

pub(super) fn change_document(
    params: Value,
    store: &mut DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
) -> io::Result<()> {
    let params = serde_json::from_value::<DidChangeParams>(params).map_err(invalid_params)?;
    let changes = params.content_changes.into_iter().map(Into::into).collect::<Vec<_>>();
    if let Err(error) =
        store.change(&params.text_document.uri, params.text_document.version, &changes)
    {
        eprintln!("actus lsp: {error}");
        return Ok(());
    }
    publish(output, &params.text_document.uri, store, target)
}

pub(super) fn close_document(
    params: Value,
    store: &mut DocumentStore,
    output: &mut impl Write,
) -> io::Result<()> {
    if let Some(uri) =
        params.get("textDocument").and_then(|document| document.get("uri")).and_then(Value::as_str)
    {
        store.close(uri);
        return write_message(
            output,
            &serde_json::to_value(Notification {
                jsonrpc: "2.0",
                method: "textDocument/publishDiagnostics",
                params: PublishDiagnosticsParams { uri: uri.to_owned(), diagnostics: Vec::new() },
            })
            .map_err(invalid_params)?,
        );
    }
    Ok(())
}

fn publish(
    output: &mut impl Write,
    uri: &str,
    store: &DocumentStore,
    target: &TargetSpec,
) -> io::Result<()> {
    let overlays = store.source_overlays();
    let diagnostics = store
        .get(uri)
        .map(|document| analyze_document_for_target(uri, &document.text, &overlays, target))
        .unwrap_or_default();
    write_message(
        output,
        &serde_json::to_value(Notification {
            jsonrpc: "2.0",
            method: "textDocument/publishDiagnostics",
            params: PublishDiagnosticsParams { uri: uri.to_owned(), diagnostics },
        })
        .map_err(invalid_params)?,
    )
}

pub(super) fn configure_target(params: &Value, target: &mut TargetSpec, store: &mut DocumentStore) {
    let requested = params
        .get("initializationOptions")
        .and_then(|options| options.get("target"))
        .and_then(Value::as_str)
        .or_else(|| params.get("target").and_then(Value::as_str));
    let Some(requested) = requested else { return };
    if let Ok(parsed) = TargetSpec::parse(requested) {
        *target = parsed;
        store.set_target(target);
    }
}

pub(super) fn respond(
    output: &mut impl Write,
    id: Option<Value>,
    result: Value,
    actus: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let Some(id) = id else { return Ok(()) };
    if cancellation.is_some_and(CancellationToken::is_canceled) {
        return write_error(output, id, -32800, "request canceled".to_owned(), actus);
    }
    write_message(
        output,
        &serde_json::to_value(Response { jsonrpc: "2.0", id, result, actus })
            .map_err(invalid_params)?,
    )
}

pub(super) fn write_error(
    output: &mut impl Write,
    id: Value,
    code: i32,
    message: String,
    actus: ResponseMetadata,
) -> io::Result<()> {
    let result_state = match code {
        -32601 => "unsupported",
        -32800 => "partial",
        _ => "invalid",
    };
    let actus = ResponseMetadata { result_state, ..actus };
    let response =
        ErrorResponse { jsonrpc: "2.0", id, error: JsonRpcError { code, message }, actus };
    write_message(output, &serde_json::to_value(response).map_err(invalid_params)?)
}

pub(super) fn write_lifecycle_error(
    output: &mut impl Write,
    id: Option<Value>,
    message: &str,
    actus: ResponseMetadata,
) -> io::Result<bool> {
    if let Some(id) = id {
        write_error(output, id, -32600, message.to_owned(), actus)?;
    }
    Ok(false)
}

pub(super) fn invalid_params(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
