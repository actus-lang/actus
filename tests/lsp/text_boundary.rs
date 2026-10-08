use std::fs;

use serde_json::json;

use crate::lsp_support::{file_uri, position_after, response_with_id, run_lsp, temp_root};

#[test]
fn lsp_resolves_the_public_utf8_boundary_from_the_standard_runtime() {
    let root = temp_root();
    let source_path = root.join("src/main.act");
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-text-boundary\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("write manifest");
    let source = concat!(
        "import std::string;\n",
        "verb main() -> Int {\n",
        "erg storage = Buffer[0];\n",
        "erg result = utf8_from_buffer(storage: dat storage);\n",
        "return 0;\n",
        "}\n",
    );
    fs::write(&source_path, source).expect("write source");
    let uri = file_uri(&source_path);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source,"utf8_from_buffer")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":18}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":position_after(source,"utf8_from_buffer")}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let hover = response_with_id(&stdout, 2).to_string();
    let completion = response_with_id(&stdout, 3).to_string();
    let definition = response_with_id(&stdout, 4).to_string();
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(hover.contains("utf8_from_buffer"), "hover missing: {hover}");
    assert!(completion.contains("utf8_from_buffer"), "completion missing: {completion}");
    assert!(definition.contains("library/std/src/string"), "definition missing: {definition}");
    assert!(stdout.contains("\"newText\""), "formatting missing: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_exposes_the_public_wire_surface_to_editor_clients() {
    let root = temp_root();
    let source_path = root.join("src/main.act");
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-wire-surface\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("write manifest");
    let source = concat!(
        "import std::wire;\n",
        "verb main() -> Int {\n",
        "erg input: Buffer = Buffer[0];\n",
        "erg start: u32 = 0u32;\n",
        "erg end: u32 = 0u32;\n",
        "erg result = wire_crc16_ccitt(input: abs input, start: erg start, end: erg end);\n",
        "return 0;\n",
        "}\n",
    );
    fs::write(&source_path, source).expect("write source");
    let uri = file_uri(&source_path);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source,"wire_crc16_ccitt")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":5,"character":31}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":position_after(source,"wire_crc16_ccitt")}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let hover = response_with_id(&stdout, 2).to_string();
    let completion = response_with_id(&stdout, 3).to_string();
    let definition = response_with_id(&stdout, 4).to_string();
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(hover.contains("wire_crc16_ccitt"), "hover missing: {hover}");
    assert!(completion.contains("wire_crc16_ccitt"), "completion missing: {completion}");
    assert!(definition.contains("library/std/src/wire"), "definition missing: {definition}");
    assert!(stdout.contains("\"newText\""), "formatting missing: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_exposes_the_public_region_surface_to_editor_clients() {
    let root = temp_root();
    let source_path = root.join("src/main.act");
    fs::create_dir_all(root.join("src")).expect("create source root");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-region-surface\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n\n[build]\nruntime = \"std\"\n",
    )
    .expect("write manifest");
    let source = concat!(
        "import std::region;\n",
        "verb main() -> Int {\n",
        "erg backing: Buffer = Buffer[4];\n",
        "erg length: u64 = 1u64;\n",
        "erg start: u64 = 0u64;\n",
        "erg count: u64 = 1u64;\n",
        "erg opened = region_open[u32](backing: dat backing, logical_length: erg length, window_start: erg start, window_count: erg count);\n",
        "return 0;\n",
        "}\n",
    );
    fs::write(&source_path, source).expect("write source");
    let uri = file_uri(&source_path);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source,"region_open")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":6,"character":30}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":position_after(source,"region_open")}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let hover = response_with_id(&stdout, 2).to_string();
    let completion = response_with_id(&stdout, 3).to_string();
    let definition = response_with_id(&stdout, 4).to_string();
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(hover.contains("region_open"), "hover missing: {hover}");
    assert!(completion.contains("region_open"), "completion missing: {completion}");
    assert!(definition.contains("library/std/src/region"), "definition missing: {definition}");
    assert!(stdout.contains("\"newText\""), "formatting missing: {stdout}");
    let _ = fs::remove_dir_all(root);
}
