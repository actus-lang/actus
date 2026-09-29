use std::io::{self, BufRead, Write};
use std::sync::{Arc, mpsc};
use std::thread;

use serde_json::{Value, json};

use super::cancellation::CancellationRegistry;

pub(super) fn spawn_reader(registry: Arc<CancellationRegistry>) -> mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        while let Ok(Some(message)) = read_message(&mut input) {
            if is_cancellation(&message, &registry) {
                continue;
            }
            if sender.send(message).is_err() {
                break;
            }
        }
    });
    receiver
}

fn is_cancellation(message: &[u8], registry: &CancellationRegistry) -> bool {
    let Ok(request) = serde_json::from_slice::<Value>(message) else { return false };
    if request.get("method").and_then(Value::as_str) != Some("$/cancelRequest") {
        return false;
    }
    let Some(id) = request.get("params").and_then(|params| params.get("id")) else {
        return true;
    };
    registry.cancel(id);
    true
}

pub(super) fn read_message(input: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(length) = content_length else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"));
    };
    let mut body = vec![0; length];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

pub(super) fn write_message(output: &mut impl Write, message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write!(output, "Content-Length: {}\r\n\r\n", body.len())?;
    output.write_all(&body)?;
    output.flush()
}

pub(super) fn progress_message(token: &str, value: Value) -> Value {
    json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":token,"value":value}})
}
