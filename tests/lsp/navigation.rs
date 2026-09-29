use serde_json::json;
use std::fs;

use crate::lsp_support::{file_uri, position_after, response_with_id, run_lsp, temp_root};

#[test]
fn lsp_navigation_surface_returns_spans_and_call_hierarchy() {
    let uri = "file:///tmp/actus-lsp-navigation.act";
    let source = "verb add(abs value: Int) -> Int { return value; }\nverb caller() -> Int { return add(value: 1); }\n";
    let add_position = position_after(source, "verb add");
    let caller_item = json!({
        "name":"caller","kind":12,"uri":uri,
        "range":{"start":{"line":1,"character":0},"end":{"line":1,"character":50}},
        "selectionRange":{"start":{"line":1,"character":5},"end":{"line":1,"character":11}}
    });
    let add_item = json!({
        "name":"add","kind":12,"uri":uri,
        "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":50}},
        "selectionRange":{"start":{"line":0,"character":5},"end":{"line":0,"character":8}}
    });
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/declaration","params":{"textDocument":{"uri":uri,"version":1},"position":add_position}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/typeDefinition","params":{"textDocument":{"uri":uri},"position":position_after(source, "value: Int")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/implementation","params":{"textDocument":{"uri":uri},"position":position_after(source, "add(value")}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/references","params":{"textDocument":{"uri":uri},"position":position_after(source, "add(value")}}),
        json!({"jsonrpc":"2.0","id":6,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"workspace/symbol","params":{"query":"caller"}}),
        json!({"jsonrpc":"2.0","id":8,"method":"textDocument/prepareCallHierarchy","params":{"textDocument":{"uri":uri},"position":position_after(source, "verb add")}}),
        json!({"jsonrpc":"2.0","id":9,"method":"callHierarchy/incomingCalls","params":{"item":add_item}}),
        json!({"jsonrpc":"2.0","id":10,"method":"callHierarchy/outgoingCalls","params":{"item":caller_item}}),
        json!({"jsonrpc":"2.0","id":11,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"name\":\"add\""), "stdout: {stdout}");
    assert!(stdout.contains("\"name\":\"caller\""), "stdout: {stdout}");
    assert!(stdout.contains("\"fromRanges\""), "stdout: {stdout}");
    assert!(stdout.contains("\"to\""), "stdout: {stdout}");
    assert!(stdout.contains("\"selectionRange\""), "stdout: {stdout}");
}

#[test]
fn lsp_navigation_rejects_stale_and_missing_definitions_and_reports_duplicates() {
    let uri = "file:///tmp/actus-lsp-navigation-negative.act";
    let source =
        "verb duplicate() -> Int { return 1; }\nverb duplicate() -> Int { return missing(); }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":2,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/declaration","params":{"textDocument":{"uri":uri,"version":1},"position":position_after(source, "missing")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/declaration","params":{"textDocument":{"uri":uri},"position":position_after(source, "missing")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("E1024"), "stdout: {stdout}");
    assert!(stdout.contains("\"resultState\":\"stale\""), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":3"), "stdout: {stdout}");
    assert!(stdout.contains("\"result\":null"), "stdout: {stdout}");
}

#[test]
fn lsp_references_follow_definition_identity_across_local_scopes() {
    let uri = "file:///tmp/actus-lsp-reference-identity.act";
    let source = "verb add(abs value: Int) -> Int { return value; }\nverb caller(abs add: Int) -> Int { return add; }\nverb use_add() -> Int { return add(value: 1); }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{"textDocument":{"uri":uri},"position":position_after(source, "verb add")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let response = response_with_id(&run_lsp(messages.to_vec()), 2);
    let references = response.get("result").and_then(|result| result.as_array()).unwrap();
    assert_eq!(references.len(), 2, "references: {response}");
}

#[test]
fn lsp_navigation_reports_ambiguous_module_roots() {
    let root = temp_root();
    let source_root = root.join("src");
    fs::create_dir_all(source_root.join("math")).expect("create ambiguous module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-ambiguous\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(source_root.join("math.act"), "open verb file_root() -> Int { return 1; }\n")
        .expect("write module file root");
    fs::write(
        source_root.join("math/math.act"),
        "open verb directory_root() -> Int { return 2; }\n",
    )
    .expect("write module directory facade");
    let main_path = source_root.join("main.act");
    let main_uri = file_uri(&main_path);
    let source = "import math;\nverb main() -> Int { return file_root(); }\n";
    fs::write(&main_path, source).expect("write main");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":main_uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("both directory"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}
