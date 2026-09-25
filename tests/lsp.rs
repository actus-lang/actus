use std::io::Write;
use std::process::{Command, Stdio};

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
