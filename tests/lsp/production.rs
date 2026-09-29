use serde_json::{Value, json};

use crate::lsp_support::{response_with_id, run_lsp};

#[test]
fn public_response_families_match_the_versioned_contract_snapshot() {
    let uri = "file:///tmp/actus-lsp-production.act";
    let source = "verb main() -> Int { return 1; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/hover","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let actual = [1, 2, 3, 4, 5].map(|id| response_projection(&response_with_id(&stdout, id)));
    let expected: Value = serde_json::from_str(include_str!(
        "../fixtures/lsp/snapshots/public_response_contracts.json"
    ))
    .expect("valid public response snapshot");
    assert_eq!(Value::Array(actual.into_iter().collect()), expected);
}

#[test]
fn diagnostics_notification_matches_the_public_contract_snapshot() {
    let uri = "file:///tmp/actus-lsp-production-diagnostics.act";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":7,"text":"verb main() -> Int { return missing; }\n"}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let notification = protocol_messages(&stdout)
        .into_iter()
        .find(|message| message.get("method") == Some(&json!("textDocument/publishDiagnostics")))
        .expect("diagnostics notification");
    let diagnostics = notification["params"]["diagnostics"].as_array().expect("diagnostic list");
    let actual = json!({
        "method": notification["method"],
        "params_keys": sorted_keys(&notification["params"]),
        "diagnostic_keys": sorted_keys(&diagnostics[0]),
        "version": notification["params"]["version"],
    });
    let expected: Value = serde_json::from_str(include_str!(
        "../fixtures/lsp/snapshots/diagnostics_notification.json"
    ))
    .expect("valid diagnostics snapshot");
    assert_eq!(actual, expected);
}

fn response_projection(response: &Value) -> Value {
    let metadata = response.get("actus").expect("response metadata");
    json!({
        "id": response["id"],
        "jsonrpc": response["jsonrpc"],
        "protocol_version": metadata["protocolVersion"],
        "schema_version": metadata["schemaVersion"],
        "result_state": metadata["resultState"],
        "result_kind": response.get("result").map_or("error", json_kind),
        "error_code": response.get("error").and_then(|error| error.get("code")),
    })
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn sorted_keys(value: &Value) -> Vec<&str> {
    let mut keys =
        value.as_object().expect("object response").keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}

fn protocol_messages(stdout: &str) -> Vec<Value> {
    stdout
        .split("Content-Length: ")
        .skip(1)
        .filter_map(|frame| frame.split_once("\r\n\r\n"))
        .filter_map(|(_, body)| serde_json::from_str(body).ok())
        .collect()
}
