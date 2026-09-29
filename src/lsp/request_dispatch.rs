use std::io::{self, Write};

use serde_json::Value;

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::protocol::{Request, ResponseMetadata};
use super::protocol_contract::{
    SessionState, initialize_result, response_metadata, validate_initialize_contract,
};
use crate::target::TargetSpec;

pub(super) fn dispatch_request(
    request: Request,
    store: &mut DocumentStore,
    target: &mut TargetSpec,
    state: &mut SessionState,
    output: &mut impl Write,
    cancellation: Option<&CancellationToken>,
) -> io::Result<bool> {
    let metadata = response_metadata(target, &request.params, store);
    if is_query_method(&request.method) {
        let mut query = QueryContext { store, target, output, metadata, cancellation };
        query_request(&request.method, request.id, request.params, &mut query)?;
        return Ok(false);
    }
    if is_lifecycle_method(&request.method) {
        return lifecycle_request(request, store, target, state, output, cancellation);
    }
    mutation_request(request, store, target, output, metadata)
}

fn is_query_method(method: &str) -> bool {
    matches!(
        method,
        "textDocument/definition"
            | "textDocument/hover"
            | "textDocument/completion"
            | "completionItem/resolve"
            | "textDocument/signatureHelp"
            | "textDocument/semanticTokens/full"
            | "textDocument/formatting"
            | "textDocument/declaration"
            | "textDocument/typeDefinition"
            | "textDocument/implementation"
            | "textDocument/references"
            | "textDocument/documentSymbol"
            | "workspace/symbol"
            | "textDocument/prepareCallHierarchy"
            | "callHierarchy/incomingCalls"
            | "callHierarchy/outgoingCalls"
    )
}

fn is_lifecycle_method(method: &str) -> bool {
    matches!(method, "initialize" | "initialized" | "shutdown" | "exit")
}

fn lifecycle_request(
    request: Request,
    store: &mut DocumentStore,
    target: &mut TargetSpec,
    state: &mut SessionState,
    output: &mut impl Write,
    cancellation: Option<&CancellationToken>,
) -> io::Result<bool> {
    let metadata = response_metadata(target, &request.params, store);
    match request.method.as_str() {
        "initialize" => initialize_request(request, target, state, output, store, cancellation),
        "initialized" => running_notification(request, state, output, metadata),
        "shutdown" => shutdown_request(request, state, output, metadata, cancellation),
        "exit" => Ok(true),
        _ => Ok(false),
    }
}

fn initialize_request(
    request: Request,
    target: &mut TargetSpec,
    state: &mut SessionState,
    output: &mut impl Write,
    store: &mut DocumentStore,
    cancellation: Option<&CancellationToken>,
) -> io::Result<bool> {
    if *state != SessionState::Created {
        return super::server::write_lifecycle_error(
            output,
            request.id,
            "server already initialized",
            response_metadata(target, &request.params, store),
        );
    }
    validate_initialize_contract(&request.params)?;
    super::server::configure_target(&request.params, target, store);
    *state = SessionState::Running;
    super::server::respond(
        output,
        request.id,
        initialize_result(target, &request.params),
        response_metadata(target, &request.params, store),
        cancellation,
    )?;
    Ok(false)
}

fn running_notification(
    request: Request,
    state: &SessionState,
    output: &mut impl Write,
    metadata: ResponseMetadata,
) -> io::Result<bool> {
    if *state != SessionState::Running {
        return super::server::write_lifecycle_error(
            output,
            request.id,
            "server is not running",
            metadata,
        );
    }
    Ok(false)
}

fn shutdown_request(
    request: Request,
    state: &mut SessionState,
    output: &mut impl Write,
    metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<bool> {
    if *state != SessionState::Running {
        return super::server::write_lifecycle_error(
            output,
            request.id,
            "server is not running",
            metadata,
        );
    }
    *state = SessionState::ShuttingDown;
    super::server::respond(output, request.id, Value::Null, metadata, cancellation)?;
    Ok(false)
}

fn mutation_request(
    request: Request,
    store: &mut DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
    metadata: ResponseMetadata,
) -> io::Result<bool> {
    match request.method.as_str() {
        "$/cancelRequest" => {}
        "textDocument/didOpen" => {
            super::server::open_document(request.params, store, target, output)?
        }
        "textDocument/didChange" => {
            super::server::change_document(request.params, store, target, output)?
        }
        "textDocument/didClose" => super::server::close_document(request.params, store, output)?,
        _ if request.id.is_some() => super::server::write_error(
            output,
            request.id.unwrap(),
            -32601,
            "method not found".to_owned(),
            metadata,
        )?,
        _ => {}
    }
    Ok(false)
}

struct QueryContext<'a, W> {
    store: &'a DocumentStore,
    target: &'a TargetSpec,
    output: &'a mut W,
    metadata: ResponseMetadata,
    cancellation: Option<&'a CancellationToken>,
}

fn query_request<W: Write>(
    method: &str,
    id: Option<Value>,
    params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    match method {
        "textDocument/declaration"
        | "textDocument/typeDefinition"
        | "textDocument/implementation"
        | "textDocument/references"
        | "textDocument/documentSymbol"
        | "workspace/symbol"
        | "textDocument/prepareCallHierarchy"
        | "callHierarchy/incomingCalls"
        | "callHierarchy/outgoingCalls" => super::navigation::dispatch(
            method,
            id,
            params,
            context.store,
            context.output,
            context.metadata.clone(),
            context.cancellation,
        ),
        "textDocument/definition" => definition(id, params, context),
        "textDocument/hover" => hover(id, params, context),
        "textDocument/completion" => completion(id, params, context),
        "completionItem/resolve" => resolve_completion(id, params, context),
        "textDocument/signatureHelp" => signature_help(id, params, context),
        "textDocument/semanticTokens/full" => semantic_tokens_full(id, params, context),
        "textDocument/formatting" => formatting(id, params, context),
        _ => Ok(()),
    }
}

fn definition<W: Write>(
    id: Option<Value>,
    params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    super::query_handlers::definition(
        id,
        params,
        context.store,
        context.output,
        context.metadata.clone(),
        context.cancellation,
    )
}

fn hover<W: Write>(
    id: Option<Value>,
    params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    super::query_handlers::hover(
        id,
        params,
        context.store,
        context.target,
        context.output,
        context.metadata.clone(),
        context.cancellation,
    )
}

fn completion<W: Write>(
    id: Option<Value>,
    params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    super::query_handlers::completion(
        id,
        params,
        context.store,
        context.target,
        context.output,
        context.metadata.clone(),
        context.cancellation,
    )
}

fn signature_help<W: Write>(
    id: Option<Value>,
    params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    super::signature_help::dispatch(
        params,
        context.store,
        context.target,
        context.output,
        id,
        context.metadata.clone(),
        context.cancellation,
    )
}

fn resolve_completion<W: Write>(
    id: Option<Value>,
    mut params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    let stale =
        super::query_handlers::resolve_completion(&mut params, context.store, context.target);
    let mut metadata = context.metadata.clone();
    if stale {
        metadata.result_state = "stale";
    }
    super::server::respond(context.output, id, params, metadata, context.cancellation)
}

fn semantic_tokens_full<W: Write>(
    id: Option<Value>,
    params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    super::query_handlers::semantic_tokens_full(
        id,
        params,
        context.store,
        context.target,
        context.output,
        context.metadata.clone(),
        context.cancellation,
    )
}

fn formatting<W: Write>(
    id: Option<Value>,
    params: Value,
    context: &mut QueryContext<'_, W>,
) -> io::Result<()> {
    super::query_handlers::formatting(
        id,
        params,
        context.store,
        context.output,
        context.metadata.clone(),
        context.cancellation,
    )
}

pub(super) fn is_expensive(method: &str) -> bool {
    is_query_method(method)
}
