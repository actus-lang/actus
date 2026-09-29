use serde_json::json;

use crate::lsp_support::{position_after, response_with_id, run_lsp};

#[test]
fn lsp_completion_filters_target_specific_declarations() {
    let uri = "file:///tmp/actus-lsp-target-completion.act";
    let source = "meta target(\"windows\")\nverb windows_only() -> Int { return 1; }\nmeta target(\"unix\")\nverb unix_only() -> Int { return 2; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"target":"x86_64-unknown-linux-gnu"}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":4,"character":0}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "windows_only")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("unix_only"), "stdout: {stdout}");
    assert!(response_with_id(&stdout, 3).get("result").is_some_and(serde_json::Value::is_null));
}

#[test]
fn lsp_hover_renders_compiler_binding_type_and_access_state() {
    let uri = "file:///tmp/actus-lsp-binding-model.act";
    let source = "verb main() -> Int { erg count: Int = 1; return count; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "count:")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("erg count: Int"), "stdout: {stdout}");
    assert!(stdout.contains("ownership: Moved"), "stdout: {stdout}");
    assert!(stdout.contains("access: Mutable"), "stdout: {stdout}");
    assert!(stdout.contains("erg count: Int [Moved; Mutable]"), "stdout: {stdout}");
}
