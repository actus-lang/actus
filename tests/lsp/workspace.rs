use serde_json::json;
use std::fs;

use crate::lsp_support::{file_uri, run_lsp, temp_root};

#[test]
fn lsp_reports_monotonic_workspace_versions_for_overlay_updates() {
    let uri = "file:///tmp/actus-lsp-workspace-version.act";
    let source = "verb stable() -> Int { return 7; }\n";
    let changed = "verb changed() -> Int { return 8; }\n";
    let latest = "verb latest() -> Int { return 9; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":changed}]}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":3},"contentChanges":[{"text":latest}]}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"workspaceVersion\":1"), "stdout: {stdout}");
    assert!(stdout.contains("\"workspaceVersion\":2"), "stdout: {stdout}");
    assert!(stdout.contains("\"workspaceVersion\":3"), "stdout: {stdout}");
    assert!(stdout.contains("\"version\":2"), "stdout: {stdout}");
    assert!(stdout.contains("changed"), "stdout: {stdout}");
    assert!(stdout.contains("latest"), "stdout: {stdout}");
}

#[test]
fn lsp_rejects_invalid_incremental_changes_without_partial_overlay_mutation() {
    let uri = "file:///tmp/actus-lsp-atomic-change.act";
    let source = "verb stable() -> Int { return 7; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"range":{"start":{"line":99,"character":0},"end":{"line":99,"character":1}},"text":"broken"}]}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("stable"), "stdout: {stdout}");
    assert!(stdout.contains("\"version\":1"), "stdout: {stdout}");
    assert!(!stdout.contains("\"version\":2"), "stdout: {stdout}");
}

#[test]
fn lsp_removes_closed_overlay_and_falls_back_to_disk_module() {
    let root = temp_root();
    let module = root.join("src/math");
    fs::create_dir_all(&module).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-close\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(module.join("math.act"), "open api;\n").expect("write facade");
    let main_path = root.join("src/main.act");
    let api_path = module.join("api.act");
    let main_uri = file_uri(&main_path);
    let api_uri = file_uri(&api_path);
    let main = "import math;\nverb main() -> Int { return disk_only(); }\n";
    let disk_api = "open verb disk_only() -> Int { return 3; }\n";
    let overlay_api = "open verb unsaved_only() -> Int { return 4; }\n";
    fs::write(&main_path, main).expect("write main");
    fs::write(&api_path, disk_api).expect("write api");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":main_uri,"version":1,"text":main}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":api_uri,"version":1,"text":overlay_api}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":api_uri}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":main_uri},"position":{"line":1,"character":30}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("disk_only"), "stdout: {stdout}");
    assert!(!stdout.contains("unsaved_only"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_reindexes_facade_dependency_and_renamed_sibling_changes() {
    let root = temp_root();
    let module = root.join("src/math");
    fs::create_dir_all(&module).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-reindex\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(module.join("math.act"), "open api;\n").expect("write facade");
    let main_path = root.join("src/main.act");
    let api_path = module.join("api.act");
    let renamed_path = module.join("renamed.act");
    let main_uri = file_uri(&main_path);
    let facade_uri = file_uri(&module.join("math.act"));
    let api_uri = file_uri(&api_path);
    let renamed_uri = file_uri(&renamed_path);
    let main = "import math;\nverb main() -> Int { return renamed_symbol(); }\n";
    let facade = "open api;\n";
    let api = "open verb old_symbol() -> Int { return 1; }\n";
    let renamed = "open verb renamed_symbol() -> Int { return 2; }\n";
    fs::write(&main_path, main).expect("write main");
    fs::write(&api_path, api).expect("write api");
    fs::write(&renamed_path, renamed).expect("write renamed sibling");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":main_uri,"version":1,"text":main}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":facade_uri,"version":1,"text":facade}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":api_uri,"version":1,"text":api}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":api_uri,"version":2},"contentChanges":[{"text":"open verb changed_symbol() -> Int { return 3; }\n"}]}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":facade_uri,"version":2},"contentChanges":[{"text":"open renamed;\n"}]}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":api_uri}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":renamed_uri,"version":1,"text":renamed}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":main_uri},"position":{"line":1,"character":35}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("renamed_symbol"), "stdout: {stdout}");
    assert!(!stdout.contains("old_symbol"), "stdout: {stdout}");
    assert!(!stdout.contains("changed_symbol"), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_rejects_oversized_document_overlays() {
    let uri = "file:///tmp/actus-lsp-oversized.act";
    let source = "x".repeat(1024 * 1024 + 1);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"resultState\":\"unsupported\""), "stdout: {stdout}");
    assert!(stdout.contains("\"workspaceVersion\":0"), "stdout: {stdout}");
}
