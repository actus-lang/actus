use std::fs;

use serde_json::json;

use crate::lsp_support::{file_uri, response_with_id, run_lsp, temp_root};

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
fn lsp_accepts_statement_conditionals_and_type_directed_integer_indexing() {
    let uri = "file:///tmp/actus-lsp-ergonomics.act";
    let source = "verb main() -> Int { erg values: Array[u8, 2] = Array[u8, 2](); erg index: u32 = 0; loop { if index >= 1 { break; } values[index] = 41; index += 1; } return values[0] + 1; }";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"diagnostics\":[]"), "valid ergonomics were rejected: {stdout}");
}

#[test]
fn lsp_analyzes_nested_child_documents_through_the_outer_facade() {
    let root = temp_root();
    let child = root.join("src/aie/runtime");
    fs::create_dir_all(&child).expect("create nested module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"nested-lsp\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/main.act"), "import aie;\nverb main() -> Int { return 0; }\n")
        .expect("write package entry");
    fs::write(root.join("src/aie/aie.act"), "open runtime;\n").expect("write parent facade");
    fs::write(child.join("runtime.act"), "open engine;\n").expect("write child facade");
    let engine = child.join("engine.act");
    let source = "open verb runtime_value() -> Int { return 42; }\n";
    fs::write(&engine, source).expect("write nested implementation");
    let uri = file_uri(&engine);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(!stdout.contains("E1108"), "nested child was treated as a root: {stdout}");
    assert!(stdout.contains("\"diagnostics\":[]"), "nested child diagnostics failed: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_reports_pack_storage_contracts_with_stable_codes_and_spans() {
    let uri = "file:///tmp/actus-lsp-pack-diagnostics.act";
    let source = "pack Control { erg storage: Array[u16, 2]; layout little; fields { erg first: u8 at 0; } }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"code\":\"E1070\""), "storage diagnostic missing: {stdout}");
    assert!(
        stdout.contains("pack `Control` requires unsigned fixed storage"),
        "storage message missing: {stdout}"
    );
    assert!(stdout.contains("\"range\":{\"end\":{\"character\":41,\"line\":0},\"start\":{\"character\":28,\"line\":0}"), "storage span drifted: {stdout}");
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
