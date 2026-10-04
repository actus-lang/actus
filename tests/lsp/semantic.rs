use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::json;

use crate::lsp_support::{file_uri, frame, position_after, response_with_id, run_lsp, temp_root};

#[test]
fn lsp_covers_current_ownership_surface_and_diagnostics() {
    assert_lsp_surface_support();
    assert_lsp_ownership_diagnostics();
}

#[test]
fn lsp_supports_the_public_monotonic_time_declarations() {
    let uri = "file:///tmp/actus-lsp-monotonic-time.act";
    let source = concat!(
        "open struct Duration { erg nanos: u64, }\n",
        "open struct Deadline { erg expires_at: u64, }\n",
        "open verb monotonic_nanos() -> u64 { return 0u64; }\n",
        "open verb duration_nanos(erg nanos: u64) -> Duration { return Duration { nanos: nanos, }; }\n",
        "open verb deadline_after(abs start: u64, abs duration: Duration) -> Deadline { return Deadline { expires_at: start + duration.nanos, }; }\n",
        "verb main() -> Int { return 0; }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source,"monotonic_nanos")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(stdout.contains("verb monotonic_nanos() -> u64"), "hover missing: {stdout}");
    assert!(stdout.contains("\"newText\""), "formatting response missing: {stdout}");
}

#[test]
fn lsp_filters_target_declarations_and_marks_inactive_code() {
    let uri = "file:///tmp/actus-lsp-targets.act";
    let source = "meta target(\"unix\") verb platform_value() -> Int { return 41; } meta target(\"windows\") verb platform_value() -> Int { return 99; } verb main() -> Int { return platform_value(); }";
    let messages = [
        json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{"initializationOptions":{"target":"x86_64-pc-windows-msvc"}}
        }),
        json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0",
            "id":2,
            "method":"textDocument/semanticTokens/full",
            "params":{"textDocument":{"uri":uri}}
        }),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(stdout.contains("\"tokenModifiers\":[\"inactive-target\"]"), "stdout: {stdout}");
    assert!(stdout.contains(",1,"), "stdout: {stdout}");
}

fn assert_lsp_surface_support() {
    let uri = "file:///tmp/actus-lsp-surface.act";
    let source = "\"\"\"Mutates a buffer.\"\"\"\nverb mutate(ins buffer: Buffer) { }\nverb slice(abs input: Buffer) -> abs Buffer { return input; }\nunsafe extern \"C\" verb host(erg code: Int) -> Int;\nverb main(abs ready: Bool) -> Int { erg buffer = Buffer[4]; erg code = 0; mutate(buffer: ins buffer); abs view = ref slice(input: abs buffer); return case abs ready { true if ready => host(code: erg code), _ => 1, }; }\n";
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
    let source = "\"\"\"ქართული 🚀\"\"\"\nverb greet() -> Int { return \"გამარჯობა 🚀\"; }\n";
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
fn lsp_keeps_systems_syntax_parity_for_generics_booleans_and_fields() {
    let uri = "file:///tmp/actus-lsp-systems-parity.act";
    let source = "struct Fabric[N: Usize] { cells: Array[Int, N], }\nverb inspect(erg fabric: Fabric[4]) -> Int { return fabric.cells[0]; }\nverb main() -> Bool { erg ready: Bool = false; if ready { return true; } return false; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "N")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "false")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":position_after(source, "fabric.cells")}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":0}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let generic_hover = response_with_id(&stdout, 2).to_string();
    let boolean_hover = response_with_id(&stdout, 3).to_string();
    let definition = response_with_id(&stdout, 4).to_string();
    assert!(generic_hover.contains("const N: Usize"), "generic hover missing: {generic_hover}");
    assert!(boolean_hover.contains("type Bool"), "boolean hover missing: {boolean_hover}");
    assert!(definition.contains("range"), "field definition missing: {definition}");
    assert!(stdout.contains("\"label\":\"true\""), "boolean completion missing: {stdout}");
    assert!(stdout.contains("\"boolean\""), "boolean semantic token legend missing: {stdout}");
}

#[test]
fn lsp_understands_the_phase23_readiness_source_without_overlay_drift() {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/phase23_readiness");
    let path = root.join("src/main.act");
    let source = std::fs::read_to_string(&path).expect("read Phase 23.8 source");
    let uri = file_uri(&path);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"diagnostics\":[]"), "readiness diagnostics drifted: {stdout}");
    assert!(stdout.contains("\"id\":2"), "readiness formatting response missing: {stdout}");
    assert!(stdout.contains("\"id\":3"), "readiness semantic token response missing: {stdout}");
}

#[test]
fn lsp_exposes_gate_7_and_gate_8_indexing_and_casts() {
    let uri = "file:///tmp/actus-lsp-gate8.act";
    let source = "verb main() -> Int { erg values: Array[u8, 2] = Array[u8, 2](); erg index: Usize = 0; erg bytes: Buffer = Buffer[0]; append(bytes, 41); erg byte_index: u8 = 0; return (values[index] as Int) + (bytes[byte_index] as Int); }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"label\":\"Array\""), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"Arena\""), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"Result\""), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"Usize\""), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"as\""), "stdout: {stdout}");
    assert!(stdout.contains("\"operator\""), "stdout: {stdout}");
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
    assert!(stdout.contains("pack ControlRegister field"), "pack field metadata missing: {stdout}");
    assert!(stdout.contains("offset: 0"), "pack field offset missing: {stdout}");
    assert!(stdout.contains("pack-keyword"), "stdout: {stdout}");
}

#[test]
fn lsp_exposes_adr44_operators_and_length_aware_buffer_hover() {
    let uri = "file:///tmp/actus-lsp-adr44.act";
    let source = "verb main() -> Bool { erg left: u8 = 1; erg right: u8 = 2; erg text = Buffer[1]; append(text, 65); print(text); return left < right && left != 0; }\n";
    let operator_position = position_after(source, "<");
    let print_position = position_after(source, "print");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":operator_position}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":print_position}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":uri}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("relational operator -> Bool"), "stdout: {stdout}");
    assert!(stdout.contains("Writes exactly the Buffer live length"), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"&&\""), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"%\""), "stdout: {stdout}");
    assert!(stdout.contains("\"operator\""), "stdout: {stdout}");
}

#[test]
fn lsp_publishes_stable_adr44_diagnostics() {
    let diagnostics = [
        (
            "file:///tmp/actus-lsp-adr44-errors.act",
            "verb main() -> Int { return 7 % 0; }\n",
            "E1092",
        ),
        ("file:///tmp/actus-lsp-adr43-errors.act", "verb main() { break; }\n", "E1020"),
    ];
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":diagnostics[0].0,"version":1,"text":diagnostics[0].1}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":diagnostics[1].0,"version":1,"text":diagnostics[1].1}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    for (_, _, code) in diagnostics {
        assert!(stdout.contains(code), "stdout: {stdout}");
    }
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
fn lsp_renders_arena_capacity_and_place_signature() {
    let uri = "file:///tmp/actus-lsp-arena.act";
    let source = "struct Node { value: Int, }\nverb main() -> Int { erg arena: Arena[64] = Arena[64] {}; erg node = arena.place(Node { value: 7, }); return node.value; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "arena.place")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("Arena[64].place(value) -> ins T"), "stdout: {stdout}");
    assert!(stdout.contains("aligned bump-pointer storage"), "stdout: {stdout}");
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

#[test]
fn lsp_resolves_std_fs_imports_for_open_documents() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("library/std/src/fs/metadata.act");
    let source = fs::read_to_string(&path).expect("read std fs metadata source");
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

#[test]
fn lsp_resolves_standard_library_test_fixture_imports() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fs_path = root.join("tests/library/fs_contracts.act");
    let path_path = root.join("tests/library/path_components.act");
    let fs_uri = file_uri(&fs_path);
    let path_uri = file_uri(&path_path);
    let fs_source = fs::read_to_string(&fs_path).expect("read fs fixture");
    let path_source = fs::read_to_string(&path_path).expect("read path fixture");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":fs_uri,"version":1,"text":fs_source}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":path_uri,"version":1,"text":path_source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"diagnostics\":[]"), "stdout: {stdout}");
    assert!(!stdout.contains("E1023"), "stdout: {stdout}");
    assert!(!stdout.contains("E1045"), "stdout: {stdout}");
}

#[test]
fn lsp_uses_unsaved_sibling_overlay_for_package_diagnostics() {
    let root = temp_root();
    let module = root.join("src/math");
    fs::create_dir_all(&module).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-overlay\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(module.join("math.act"), "open ops; open consumer;\n").expect("write facade");
    let ops_uri = file_uri(&module.join("ops.act"));
    let consumer_uri = file_uri(&module.join("consumer.act"));
    let ops = "open verb add() -> Int { return 1; }\n";
    let consumer = "open verb use() -> Int { return add(); }\n";
    fs::write(module.join("ops.act"), ops).expect("write ops");
    fs::write(module.join("consumer.act"), consumer).expect("write consumer");
    let changed_ops = "open verb renamed() -> Int { return missing; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":ops_uri,"version":1,"text":ops}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":consumer_uri,"version":1,"text":consumer}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":ops_uri,"version":2},"contentChanges":[{"text":changed_ops}]}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":consumer_uri,"version":2},"contentChanges":[{"text":consumer}]}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(
        stdout.contains("E1069"),
        "overlay diagnostics did not use unsaved sibling text: {stdout}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_external_views_use_facade_exports_and_unsaved_definitions() {
    let root = temp_root();
    let module = root.join("src/math");
    fs::create_dir_all(&module).expect("create module directory");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-interface\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    fs::write(module.join("math.act"), "open api;\n").expect("write facade");
    let main_path = root.join("src/main.act");
    let api_path = module.join("api.act");
    let main_uri = file_uri(&main_path);
    let api_uri = file_uri(&api_path);
    let main = "import math;\nverb main() -> Int { return visible(); }\n";
    let api =
        "open verb visible() -> Int { return 7; }\nverb private_bridge() -> Int { return 9; }\n";
    fs::write(&main_path, main).expect("write main");
    fs::write(&api_path, "verb disk_only() -> Int { return 3; }\n").expect("write api");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":main_uri,"version":1,"text":main}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":api_uri,"version":1,"text":api}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":main_uri,"version":2},"contentChanges":[{"text":main}]}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":main_uri},"position":{"line":1,"character":30}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":main_uri},"position":position_after(main, "visible")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/definition","params":{"textDocument":{"uri":main_uri},"position":position_after(main, "visible")}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":main_uri}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"label\":\"visible\""), "public completion missing: {stdout}");
    assert!(
        !stdout.contains("\"label\":\"private_bridge\""),
        "private completion leaked: {stdout}"
    );
    assert!(stdout.contains("verb visible() -> Int"), "public hover missing: {stdout}");
    assert!(stdout.contains(&api_uri), "definition did not use the module source: {stdout}");
    assert!(
        !stdout.contains("private_bridge"),
        "private declaration leaked into LSP output: {stdout}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn lsp_defines_exports_from_a_nested_child_facade() {
    let root = temp_root();
    let child = root.join("src/device/runtime");
    fs::create_dir_all(&child).expect("create nested child module");
    fs::write(
        root.join("Actus.toml"),
        "[package]\nname = \"lsp-nested-facade\"\nversion = \"0.1.0\"\nedition = \"alpha\"\n",
    )
    .expect("write manifest");
    let main_path = root.join("src/main.act");
    let engine_path = child.join("engine.act");
    let main_uri = file_uri(&main_path);
    let engine_uri = file_uri(&engine_path);
    let main = "import device;\nverb main() -> Int { return runtime_value(); }\n";
    let engine = "open verb runtime_value() -> Int { return 42; }\n";
    fs::write(&main_path, main).expect("write main");
    fs::write(root.join("src/device/device.act"), "open runtime;\n").expect("write parent facade");
    fs::write(child.join("runtime.act"), "open engine;\n").expect("write child facade");
    fs::write(&engine_path, "open verb runtime_value() -> Int { return 42; }\n")
        .expect("write nested export");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":main_uri,"version":1,"text":main}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":engine_uri,"version":1,"text":engine}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":main_uri},"position":position_after(main,"runtime_value")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":main_uri},"position":position_after(main,"runtime_value")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/references","params":{"textDocument":{"uri":main_uri},"position":position_after(main,"runtime_value")}}),
        json!({"jsonrpc":"2.0","id":5,"method":"textDocument/rename","params":{"textDocument":{"uri":main_uri},"position":position_after(main,"runtime_value"),"newName":"renamed_runtime_value"}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains(&engine_uri), "nested definition missing: {stdout}");
    assert!(stdout.contains("verb runtime_value() -> Int"), "nested hover missing: {stdout}");
    assert!(stdout.contains("renamed_runtime_value"), "nested rename missing: {stdout}");
    let _ = fs::remove_dir_all(root);
}
