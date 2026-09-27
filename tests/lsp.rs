use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

fn frame(message: Value) -> Vec<u8> {
    let body = serde_json::to_vec(&message).expect("serialize message");
    format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes().into_iter().chain(body).collect()
}

#[test]
fn lsp_lifecycle_publishes_and_clears_live_diagnostics() {
    let uri = "file:///tmp/actus-lsp.act";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
        json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":"verb main() -> Int { return missing; }"}}
        }),
        json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didChange",
            "params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":"verb main() -> Int { return 0; }"}]}
        }),
        json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didClose",
            "params":{"textDocument":{"uri":uri}}
        }),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
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
    let stdout = String::from_utf8(output.stdout).expect("utf8 protocol output");
    assert!(stdout.contains("\"method\":\"textDocument/publishDiagnostics\""));
    assert!(stdout.contains("E1003"));
    assert!(stdout.contains("\"diagnostics\":[]"));
    assert!(stdout.contains("\"id\":1"));
    assert!(stdout.contains("\"id\":2"));
}

#[test]
fn lsp_definition_resolves_local_and_facade_exported_symbols() {
    let root = temp_root();
    fs::create_dir_all(root.join("src/math")).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-demo\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(root.join("src/math/math.act"), "open ops;\n").expect("write facade");
    let ops = "open verb add(erg left: Int, erg right: Int) -> Int { return left + right; }\n";
    fs::write(root.join("src/math/ops.act"), ops).expect("write sibling");
    let source = "import math;\nverb main() -> Int { erg number = add(left: 40, right: 2); return number; }\n";
    fs::write(root.join("src/main.act"), source).expect("write main");
    let uri = file_uri(&root.join("src/main.act"));
    let ops_uri = file_uri(&root.join("src/math/ops.act"));
    let local_position = position_after(source, "return number");
    let external_position = position_after(source, "= add");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/definition",
            "params":{"textDocument":{"uri":uri},"position":local_position}
        }),
        json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/definition",
            "params":{"textDocument":{"uri":uri},"position":external_position}
        }),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
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
    let stdout = String::from_utf8(output.stdout).expect("utf8 protocol output");
    assert!(stdout.contains("\"id\":2") && stdout.contains(&uri), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":3") && stdout.contains(&ops_uri), "stdout: {stdout}");
    assert!(stdout.contains(&ops_uri), "stdout: {stdout}");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_hover_and_formatting_return_compiler_information() {
    let uri = "file:///tmp/actus-lsp-hover.act";
    let source = "\"\"\"Adds a value.\nThe value is inspected without ownership transfer.\n\"\"\"\nverb add(abs value: Int) -> Int { return value; }\n";
    let verb_position = position_after(source, "verb add");
    let parameter_position = position_after(source, "return value");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":verb_position}
        }),
        json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":parameter_position}
        }),
        json!({
            "jsonrpc":"2.0","id":4,"method":"textDocument/formatting",
            "params":{"textDocument":{"uri":uri}}
        }),
        json!({"jsonrpc":"2.0","id":5,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
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
    let stdout = String::from_utf8(output.stdout).expect("utf8 protocol output");
    assert!(stdout.contains("markdown"));
    assert!(stdout.contains("abs value: Int"));
    assert!(stdout.contains("Adds a value."));
    assert!(stdout.contains("The value is inspected without ownership transfer."));
    assert!(stdout.contains("newText"));
    assert!(stdout.contains("verb add(abs value: Int) -> Int"));
}

#[test]
fn lsp_covers_current_ownership_surface_and_diagnostics() {
    assert_lsp_surface_support();
    assert_lsp_ownership_diagnostics();
}

fn assert_lsp_surface_support() {
    let uri = "file:///tmp/actus-lsp-surface.act";
    let source = "/// Mutates a buffer.\nverb mutate(ins buffer: Buffer) { }\nverb slice(abs input: Buffer) -> abs Buffer { return input; }\nunsafe extern \"C\" verb host(erg code: Int) -> Int;\nverb main(abs ready: Bool) -> Int { erg buffer = Buffer[4]; erg code = 0; mutate(buffer: ins buffer); abs view = ref slice(input: abs buffer); return case abs ready { true if ready => host(code: erg code), _ => 1, }; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":position_after(source, "verb mutate")}
        }),
        json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":position_after(source, "verb slice")}
        }),
        json!({
            "jsonrpc":"2.0","id":4,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":position_after(source, "verb host")}
        }),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(stdout.contains("ins buffer: Buffer"), "stdout: {stdout}");
    assert!(stdout.contains("verb slice(abs input: Buffer) -> abs Buffer"), "stdout: {stdout}");
    assert!(stdout.contains("extern verb host(erg code: Int) -> Int"), "stdout: {stdout}");
    assert!(stdout.contains("\"newText\""), "stdout: {stdout}");
}

fn assert_lsp_ownership_diagnostics() {
    let diagnostics = [
        (
            "file:///tmp/actus-lsp-ins-alias.act",
            "verb merge(ins left: Buffer, abs view: Buffer) { } verb main() { erg buffer = Buffer[1]; merge(left: ins buffer, view: abs buffer); }",
            "E1065",
        ),
        (
            "file:///tmp/actus-lsp-abs-origin.act",
            "verb invalid(abs input: Buffer) -> abs Buffer { erg temporary = Buffer[4]; return temporary; }",
            "E1066",
        ),
        (
            "file:///tmp/actus-lsp-frozen-owner.act",
            "extern \"C\" verb slice(abs input: Buffer) -> abs Buffer; verb consume(dat input: Buffer) { drop(input); } verb main() -> Int { erg buffer = Buffer[8]; abs view = ref slice(input: abs buffer); consume(input: dat buffer); return 0; }",
            "E1011",
        ),
    ];
    let messages = std::iter::once(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))
        .chain(diagnostics.iter().map(|(uri, text, _)| {
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":text}}})
        }))
        .chain([
            json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
            json!({"jsonrpc":"2.0","method":"exit","params":null}),
        ])
        .collect::<Vec<_>>();
    let stdout = run_lsp(messages);
    for (_, _, code) in diagnostics {
        assert!(stdout.contains(code), "missing {code} in {stdout}");
    }
}

#[test]
fn lsp_preserves_utf16_ranges_with_multibyte_source_text() {
    let uri = "file:///tmp/actus-lsp-unicode.act";
    let source = "/// ქართული 🚀\nverb greet() -> Int { return \"გამარჯობა 🚀\"; }\n";
    let verb_position = position_after(source, "verb greet");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":verb_position}
        }),
        json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/formatting",
            "params":{"textDocument":{"uri":uri}}
        }),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
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
    let stdout = String::from_utf8(output.stdout).expect("utf8 protocol output");
    assert!(stdout.contains("\"start\":{\"character\":5,\"line\":1}"), "stdout: {stdout}");
    assert!(stdout.contains("\"end\":{\"character\":10,\"line\":1}"), "stdout: {stdout}");
    assert!(stdout.contains("გამარჯობა 🚀"), "stdout: {stdout}");
}

#[test]
fn lsp_exposes_gate_36_primitives_in_hover_completion_and_tokens() {
    let uri = "file:///tmp/actus-lsp-primitives.act";
    let source =
        "verb sample(erg count: u8, erg ratio: f64, erg unit: Void) -> u128 { return 0x2A; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "u8")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("completionProvider"));
    assert!(stdout.contains("type u8"));
    assert!(stdout.contains("\"label\":\"u128\""));
    assert!(stdout.contains("semanticTokensProvider"));
    assert!(stdout.contains("\"data\":["));
}

#[test]
fn lsp_exposes_pack_registers_in_hover_completion_and_tokens() {
    let uri = "file:///tmp/actus-lsp-pack.act";
    let source = "pack ControlRegister { erg storage: u32; layout little; fields { erg enabled: u1 at 0; abs ready: u1 at 1; abs _reserved: u30 at 2 = 0; } }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "ControlRegister")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("pack ControlRegister"), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"enabled\""), "stdout: {stdout}");
    assert!(stdout.contains("pack-keyword"), "stdout: {stdout}");
}

#[test]
fn lsp_resolves_pack_fields_and_renders_hardware_aware_hover() {
    let uri = "file:///tmp/actus-lsp-pack-field.act";
    let source = "pack Control { erg storage: u32; layout little; fields { erg prescaler: u4 at 2; abs _reserved: u26 at 6 = 0; } }\nverb main() -> Int { erg control = Control { storage: 0, }; control.prescaler = 3; return control.prescaler; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":position_after(source, "control.prescaler")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "control.prescaler")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("offset: 2"), "stdout: {stdout}");
    assert!(stdout.contains("width: 4 bits"), "stdout: {stdout}");
    assert!(stdout.contains("mask: 0x0F"), "stdout: {stdout}");
    assert!(stdout.contains("erg prescaler: u4"), "stdout: {stdout}");
    assert!(stdout.contains("\"start\":{\"character\":61,\"line\":0}"), "stdout: {stdout}");
}

#[test]
fn lsp_resolves_std_io_sibling_context_for_open_documents() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/io/reader.act");
    let source = fs::read_to_string(&path).expect("read std io reader source");
    let uri = file_uri(&path);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(!stdout.contains("E1023"), "stdout: {stdout}");
    assert!(!stdout.contains("E1056"), "stdout: {stdout}");
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
        format!("file:///{}", normalized.trim_start_matches('/'))
    } else {
        format!("file://{normalized}")
    }
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
