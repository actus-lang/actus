use std::fs;

use serde_json::json;

use crate::lsp_support::{file_uri, response_with_id, run_lsp, temp_root};

#[cfg(unix)]
use crate::lsp_support::position_after;

#[test]
fn lsp_handles_crlf_and_encoded_unicode_document_paths() {
    let root = temp_root().join("workspace space-ქართული");
    fs::create_dir_all(&root).expect("create unicode workspace");
    let path = root.join("main.act");
    let source = "\"\"\"🚀\"\"\"\r\nverb greet() -> Int { return 1; }\r\n";
    fs::write(&path, source).expect("write CRLF source");
    let uri = file_uri(&path);
    assert!(uri.contains("%20") && uri.contains("%E1%83"), "URI was not encoded: {uri}");

    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":1,"character":5}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(
        response_with_id(&stdout, 2).get("result").is_some_and(|result| !result.is_null()),
        "hover failed: {stdout}"
    );
    assert!(stdout.contains("🚀"), "Unicode documentation was lost: {stdout}");
    assert!(stdout.contains("\"line\":1"), "CRLF line mapping was lost: {stdout}");
    fs::remove_dir_all(root).expect("remove unicode workspace");
}

#[test]
fn lsp_reports_non_ascii_identifier_boundaries_without_corrupting_diagnostics() {
    let uri = "file:///tmp/actus-lsp-unicode-identifier.act";
    let source =
        "\"\"\"Unicode documentation remains valid\"\"\"\nverb café() -> Int { return 1; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("E0001"), "non-ASCII identifier was silently accepted: {stdout}");
}

#[cfg(unix)]
#[test]
fn lsp_module_resolution_preserves_case_sensitive_paths() {
    let root = temp_root();
    let source_root = root.join("src");
    fs::create_dir_all(source_root.join("Case")).expect("create case-sensitive module");
    let main_path = source_root.join("main.act");
    let facade_path = source_root.join("Case/Case.act");
    let implementation_path = source_root.join("Case/api.act");
    let source = "import Case;\nverb main() -> Int { return case_value(); }\n";
    fs::write(&facade_path, "open api;\n").expect("write facade");
    fs::write(&implementation_path, "open verb case_value() -> Int { return 42; }\n")
        .expect("write implementation");
    fs::write(&main_path, source).expect("write main");
    let main_uri = file_uri(&main_path);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":main_uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":main_uri},"position":position_after(source, "case_value")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("case_value"), "case-sensitive module was not resolved: {stdout}");
    fs::remove_dir_all(root).expect("remove case-sensitive workspace");
}
