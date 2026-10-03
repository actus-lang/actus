use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

#[test]
fn lsp_lifecycle_publishes_and_clears_live_diagnostics() {
    let stdout = run_lsp(lifecycle_messages());
    assert!(stdout.contains("\"method\":\"textDocument/publishDiagnostics\""));
    assert!(stdout.contains("E1003"));
    assert!(stdout.contains("\"diagnostics\":[]"));
    assert!(stdout.contains("\"id\":1"));
    assert!(stdout.contains("\"id\":2"));
}

fn lifecycle_messages() -> Vec<Value> {
    let uri = "file:///tmp/actus-lsp.act";
    vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":"verb main() -> Int { return missing; }"}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":"verb main() -> Int { return 0; }"}]}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ]
}

#[test]
fn lsp_definition_resolves_local_and_facade_exported_symbols() {
    let root = temp_root();
    let source = write_definition_fixture(&root);
    let uri = file_uri(&root.join("src/main.act"));
    let ops_uri = file_uri(&root.join("src/math/ops.act"));
    let messages = definition_messages(&uri, &source);
    let stdout = run_lsp(messages);
    assert!(!stdout.contains("unknown verb `add`"), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":2") && stdout.contains(&uri), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":3") && stdout.contains(&ops_uri), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_definition_resolves_a_configuration_constant_through_its_facade() {
    let root = temp_root();
    fs::create_dir_all(root.join("src/config")).expect("create config directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-config-demo\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/config/config.act"), "open values;\n").expect("write config facade");
    fs::write(root.join("src/config/values.act"), "open const LIMIT: u8 = 30u8;\n")
        .expect("write config values");
    let source =
        "import config;\nverb main() -> Int { erg number = LIMIT as Int; return number; }\n";
    let main = root.join("src/main.act");
    fs::write(&main, source).expect("write main");
    let uri = file_uri(&main);
    let stdout = run_lsp(configuration_definition_messages(&uri, source));
    let values_uri = file_uri(&root.join("src/config/values.act"));
    assert!(stdout.contains(&values_uri), "configuration definition missing: {stdout}");
    let _ = fs::remove_dir_all(root);
}

fn configuration_definition_messages(uri: &str, source: &str) -> Vec<Value> {
    let local_position = position_after(source, "return number");
    let external_position = position_after(source, "= LIMIT");
    vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":local_position}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":external_position}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ]
}

fn write_definition_fixture(root: &Path) -> String {
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-demo\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/math/math.act"), "open ops;\n").expect("write facade");
    fs::write(
        root.join("src/math/ops.act"),
        "open verb add(erg left: Int, erg right: Int) -> Int { return left + right; }\n",
    )
    .expect("write sibling");
    let source = "import math;\nverb main() -> Int { erg number = add(left: 40, right: 2); return number; }\n".to_owned();
    fs::write(root.join("src/main.act"), &source).expect("write main");
    source
}

fn definition_messages(uri: &str, source: &str) -> Vec<Value> {
    let local_position = position_after(source, "return number");
    let external_position = position_after(source, "= add");
    vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":local_position}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":external_position}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ]
}

fn frame(message: Value) -> Vec<u8> {
    let body = serde_json::to_vec(&message).expect("serialize message");
    format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes().into_iter().chain(body).collect()
}

fn run_lsp(messages: Vec<Value>) -> String {
    let input = messages.into_iter().flat_map(frame).collect::<Vec<_>>();
    let mut child = Command::new(env!("CARGO_BIN_EXE_actus"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start lsp");
    child.stdin.take().unwrap().write_all(&input).expect("write lsp input");
    let output = child.wait_with_output().expect("wait for lsp");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("utf8 protocol output")
}

fn position_after(source: &str, marker: &str) -> Value {
    let offset = source.find(marker).expect("marker in source") + marker.len() - 1;
    let line = source[..offset].bytes().filter(|byte| *byte == b'\n').count();
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    json!({"line":line,"character":offset - line_start})
}

fn temp_root() -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("actus-lsp-definition-{stamp}"))
}

fn file_uri(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.as_bytes().get(1) == Some(&b':') {
        format!("file:///{normalized}")
    } else {
        format!("file://{normalized}")
    }
}
