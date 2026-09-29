use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

pub(crate) fn frame(message: Value) -> Vec<u8> {
    let body = serde_json::to_vec(&message).expect("serialize message");
    format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes().into_iter().chain(body).collect()
}

pub(crate) fn position_after(source: &str, marker: &str) -> Value {
    let offset = source.find(marker).expect("marker in source") + marker.len() - 1;
    let line = source[..offset].bytes().filter(|byte| *byte == b'\n').count();
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    json!({"line":line,"character":offset - line_start})
}

pub(crate) fn fixture_messages(source: &str) -> Vec<Value> {
    serde_json::from_str(source).expect("valid LSP protocol fixture")
}

pub(crate) fn temp_root() -> PathBuf {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("actus-lsp-definition-{stamp}"))
}

pub(crate) fn file_uri(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.as_bytes().get(1) == Some(&b':') {
        format!("file:///{}", normalized.trim_start_matches('/'))
    } else {
        format!("file://{normalized}")
    }
}

pub(crate) fn run_lsp(messages: Vec<Value>) -> String {
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

pub(crate) fn response_with_id(stdout: &str, id: i64) -> Value {
    stdout
        .split("Content-Length: ")
        .skip(1)
        .filter_map(|frame| frame.split_once("\r\n\r\n"))
        .filter_map(|(_, body)| serde_json::from_str::<Value>(body).ok())
        .find(|message| message.get("id").and_then(Value::as_i64) == Some(id))
        .expect("response with requested id")
}
