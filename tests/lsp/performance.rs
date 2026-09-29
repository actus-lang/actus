use serde_json::json;

use crate::lsp_support::run_lsp;

#[test]
fn lsp_reuses_a_versioned_semantic_snapshot_and_invalidates_it_on_change() {
    let uri = "file:///tmp/actus-lsp-semantic-cache.act";
    let first = "verb stable() -> Int { return 1; }\n";
    let second = "verb changed() -> Int { return 2; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":first}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":second}]}}),
        json!({"jsonrpc":"2.0","id":4,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":2}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert_eq!(stdout.matches("\"state\":\"available\"").count(), 3, "stdout: {stdout}");
    assert!(stdout.contains("changed"), "updated semantic snapshot missing: {stdout}");
    assert!(stdout.contains("\"resultState\":\"stale\""), "stale snapshot accepted: {stdout}");
}

#[test]
fn lsp_cancellation_keeps_partial_results_explicit() {
    let uri = "file:///tmp/actus-lsp-semantic-cancel.act";
    let source = "verb main() -> Int { return 1; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":2}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"code\":-32800"), "cancellation error missing: {stdout}");
    assert!(stdout.contains("\"resultState\":\"partial\""), "partial state missing: {stdout}");
    assert!(!stdout.contains("\"id\":2,\"result\""), "canceled result published: {stdout}");
}
