use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::module_scope::file_uri_to_path;
use super::protocol::{LspDiagnostic, ResponseMetadata};
use crate::configuration::CompilerConfiguration;
use crate::target::TargetSpec;

pub(super) fn run(
    params: &Value,
    store: &DocumentStore,
    target: &TargetSpec,
    metadata: &mut ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> Value {
    let Some((uri, version)) = request_document(params) else {
        metadata.result_state = "invalid";
        return result("invalid", "", "missing textDocument URI or version", None, Vec::new());
    };
    let Some(document) = store.get(uri) else {
        metadata.result_state = "unsupported";
        return result("unsupported", "", "document is not open", None, Vec::new());
    };
    if document.version != version {
        metadata.result_state = "stale";
        return result("stale", "", "document version is stale", None, Vec::new());
    }
    let Some(path) = validated_path(uri) else {
        metadata.result_state = "unsupported";
        return result("unsupported", "", "workflow requires a local file URI", None, Vec::new());
    };
    if cancellation.is_some_and(CancellationToken::is_canceled) {
        metadata.result_state = "partial";
        return result("canceled", "", "workflow canceled before execution", None, Vec::new());
    }
    if let Some(response) = validate_source(uri, &path, &document.text, store, target, metadata) {
        return response;
    }
    execute(path, uri, version, cancellation, metadata)
}

fn request_document(params: &Value) -> Option<(&str, i64)> {
    let document = params.get("textDocument")?;
    Some((document.get("uri")?.as_str()?, document.get("version")?.as_i64()?))
}

fn validated_path(uri: &str) -> Option<std::path::PathBuf> {
    let path = file_uri_to_path(uri)?;
    (path.extension().and_then(|extension| extension.to_str()) == Some("act") && path.is_file())
        .then_some(path)
}

fn validate_source(
    uri: &str,
    path: &Path,
    source: &str,
    store: &DocumentStore,
    target: &TargetSpec,
    metadata: &mut ResponseMetadata,
) -> Option<Value> {
    if let Err(message) = saved_source_status(path, source) {
        metadata.result_state = "unsupported";
        return Some(result("unsupported", "", &message, None, Vec::new()));
    }
    if let Err(message) = target_status(path, target) {
        metadata.result_state = "unsupported";
        return Some(result("unsupported", "", &message, None, Vec::new()));
    }
    let diagnostics = super::diagnostics::analyze_document_for_target(
        uri,
        source,
        &store.source_overlays(),
        target,
    );
    if !diagnostics.is_empty() {
        metadata.result_state = "invalid";
        return Some(result("invalid", "", "source validation failed", None, diagnostics));
    }
    None
}

fn saved_source_status(path: &Path, source: &str) -> Result<(), String> {
    let disk_source =
        fs::read_to_string(path).map_err(|error| format!("cannot read source: {error}"))?;
    if disk_source != source {
        return Err("unsaved source must be saved before running".to_owned());
    }
    Ok(())
}

fn target_status(path: &Path, target: &TargetSpec) -> Result<(), String> {
    let configuration = CompilerConfiguration::from_input_path_read_only(path)
        .map_err(|error| error.to_string())?;
    if configuration.target().triple() != target.triple() {
        return Err("document target differs from the active LSP target".to_owned());
    }
    Ok(())
}

fn execute(
    path: std::path::PathBuf,
    uri: &str,
    version: i64,
    cancellation: Option<&CancellationToken>,
    metadata: &mut ResponseMetadata,
) -> Value {
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => return failed(metadata, format!("cannot locate actus executable: {error}")),
    };
    let mut child = match Command::new(executable)
        .arg("run")
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return failed(metadata, format!("cannot start workflow: {error}")),
    };
    loop {
        if cancellation.is_some_and(CancellationToken::is_canceled) {
            let _ = child.kill();
            let _ = child.wait();
            metadata.result_state = "partial";
            return result("canceled", "", "workflow canceled", None, Vec::new());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = match child.wait_with_output() {
                    Ok(output) => output,
                    Err(error) => {
                        return failed(metadata, format!("workflow output failed: {error}"));
                    }
                };
                let code = status.code();
                let state = if status.success() { "completed" } else { "failed" };
                return json!({"status":state,"uri":uri,"version":version,"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr),"exitCode":code,"diagnostics":[]});
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => return failed(metadata, format!("workflow status failed: {error}")),
        }
    }
}

fn failed(metadata: &mut ResponseMetadata, message: String) -> Value {
    metadata.result_state = "invalid";
    result("failed", "", &message, None, Vec::new())
}

fn result(
    status: &str,
    stdout: &str,
    stderr: &str,
    exit_code: Option<i32>,
    diagnostics: Vec<LspDiagnostic>,
) -> Value {
    json!({"status":status,"stdout":stdout,"stderr":stderr,"exitCode":exit_code,"diagnostics":diagnostics})
}
