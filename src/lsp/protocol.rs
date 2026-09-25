use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::documents::ContentChange;
use super::position::{LspPosition, LspRange};

#[derive(Clone, Debug, Deserialize)]
pub struct Request {
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionedTextDocumentIdentifier {
    pub uri: String,
    pub version: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DidOpenParams {
    #[serde(rename = "textDocument")]
    pub text_document: OpenTextDocument,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OpenTextDocument {
    pub uri: String,
    pub version: i64,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DidChangeParams {
    #[serde(rename = "textDocument")]
    pub text_document: VersionedTextDocumentIdentifier,
    #[serde(rename = "contentChanges")]
    pub content_changes: Vec<RawContentChange>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DefinitionParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentPosition,
    pub position: LspPosition,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TextDocumentPosition {
    pub uri: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FormattingParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TextEdit {
    pub range: LspRange,
    #[serde(rename = "newText")]
    pub new_text: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawContentChange {
    pub text: String,
    pub range: Option<LspRange>,
}

impl From<RawContentChange> for ContentChange {
    fn from(change: RawContentChange) -> Self {
        Self { range: change.range, text: change.text }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Response {
    pub jsonrpc: &'static str,
    pub id: Value,
    pub result: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct Notification {
    pub jsonrpc: &'static str,
    pub method: &'static str,
    pub params: PublishDiagnosticsParams,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublishDiagnosticsParams {
    pub uri: String,
    pub diagnostics: Vec<LspDiagnostic>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: u8,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
}
