use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

use super::definition::find_definition;
use super::diagnostics::analyze_document;
use super::documents::DocumentStore;
use super::protocol::{
    DefinitionParams, DidChangeParams, DidOpenParams, Notification, PublishDiagnosticsParams,
    Request, Response,
};

pub fn run_stdio() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout().lock();
    let mut store = DocumentStore::default();
    while let Some(message) = read_message(&mut input)? {
        let request = match serde_json::from_slice::<Request>(&message) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("actus lsp: invalid JSON-RPC message: {error}");
                continue;
            }
        };
        if handle_request(request, &mut store, &mut output)? {
            break;
        }
    }
    Ok(())
}

fn handle_request(
    request: Request,
    store: &mut DocumentStore,
    output: &mut impl Write,
) -> io::Result<bool> {
    match request.method.as_str() {
        "initialize" => respond(output, request.id, initialize_result())?,
        "shutdown" => respond(output, request.id, Value::Null)?,
        "exit" => return Ok(true),
        "initialized" | "$/cancelRequest" => {}
        "textDocument/didOpen" => open_document(request.params, store, output)?,
        "textDocument/didChange" => change_document(request.params, store, output)?,
        "textDocument/didClose" => close_document(request.params, store, output)?,
        "textDocument/definition" => definition(request.id, request.params, store, output)?,
        _ if request.id.is_some() => respond(output, request.id, Value::Null)?,
        _ => {}
    }
    Ok(false)
}

fn definition(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    output: &mut impl Write,
) -> io::Result<()> {
    let params = serde_json::from_value::<DefinitionParams>(params).map_err(invalid_params)?;
    let result = store
        .get(&params.text_document.uri)
        .and_then(|document| {
            find_definition(&params.text_document.uri, &document.text, &params.position)
        })
        .map(|location| json!({ "uri": location.uri, "range": location.range }))
        .unwrap_or(Value::Null);
    respond(output, id, result)
}

fn open_document(
    params: Value,
    store: &mut DocumentStore,
    output: &mut impl Write,
) -> io::Result<()> {
    let params = serde_json::from_value::<DidOpenParams>(params).map_err(invalid_params)?;
    store.open(
        params.text_document.uri.clone(),
        params.text_document.version,
        params.text_document.text,
    );
    publish(output, &params.text_document.uri, store)
}

fn change_document(
    params: Value,
    store: &mut DocumentStore,
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
    publish(output, &params.text_document.uri, store)
}

fn close_document(
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

fn publish(output: &mut impl Write, uri: &str, store: &DocumentStore) -> io::Result<()> {
    let diagnostics =
        store.get(uri).map(|document| analyze_document(&document.text)).unwrap_or_default();
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

fn initialize_result() -> Value {
    json!({
        "capabilities": {
            "textDocumentSync": 1,
            "definitionProvider": true,
            "hoverProvider": false,
            "documentFormattingProvider": false
        },
        "serverInfo": { "name": "actus-lsp", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn respond(output: &mut impl Write, id: Option<Value>, result: Value) -> io::Result<()> {
    let Some(id) = id else { return Ok(()) };
    write_message(
        output,
        &serde_json::to_value(Response { jsonrpc: "2.0", id, result }).map_err(invalid_params)?,
    )
}

fn read_message(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(length) = content_length else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"));
    };
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

fn invalid_params(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
