use serde_json::json;

use crate::lsp_support::{response_with_id, run_lsp};

#[test]
fn lsp_recovers_incomplete_source_at_document_end() {
    let uri = "file:///tmp/actus-lsp-incomplete.act";
    let source = "verb main() { erg answer = 1;";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":7,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("E0004"), "missing EOF diagnostic: {stdout}");
    assert!(stdout.contains("complete the declaration"), "missing recovery hint: {stdout}");
    assert!(stdout.contains("\"character\":29"), "EOF range was not recovered: {stdout}");
}

#[test]
fn lsp_keeps_lexical_and_type_diagnostics_stable() {
    let lexical_uri = "file:///tmp/actus-lsp-lexical.act";
    let type_uri = "file:///tmp/actus-lsp-type.act";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":lexical_uri,"version":1,"text":"verb main() { @ }"}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":type_uri,"version":1,"text":"verb main() -> Int { return \"wrong\"; }"}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("E0001"), "missing lexical code: {stdout}");
    assert!(stdout.contains("E1026"), "missing semantic code: {stdout}");
}

#[test]
fn lsp_publishes_current_version_and_clears_after_replacement_and_close() {
    let uri = "file:///tmp/actus-lsp-versioned.act";
    let invalid = "verb main() { @ }";
    let valid = "verb main() -> Int { return 1; }";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":3,"text":invalid}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":4},"contentChanges":[{"text":valid}]}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"version\":3"), "open version missing: {stdout}");
    assert!(stdout.contains("\"version\":4"), "replacement version missing: {stdout}");
    assert!(stdout.contains("\"diagnostics\":[]"), "clear notification missing: {stdout}");
    assert_eq!(
        stdout.matches("\"code\":\"E0001\"").count(),
        1,
        "stale lexical result was republished: {stdout}"
    );
    let _ = response_with_id(&stdout, 2);
}
