use std::io;

use serde_json::{Value, json};

use super::documents::DocumentStore;
use super::protocol::{
    ACTUS_LSP_PROTOCOL_VERSION, ACTUS_LSP_SCHEMA_VERSION, DocumentMetadata, ResponseMetadata,
};
use crate::target::TargetSpec;

#[derive(Default, PartialEq)]
pub(super) enum SessionState {
    #[default]
    Created,
    Running,
    ShuttingDown,
}

pub(super) fn initialize_result(target: &TargetSpec, params: &Value) -> Value {
    let negotiated = negotiated_capabilities(params);
    json!({
        "capabilities": {
            "textDocumentSync": 1,
            "definitionProvider": true,
            "declarationProvider": true,
            "typeDefinitionProvider": true,
            "implementationProvider": true,
            "referencesProvider": true,
            "documentSymbolProvider": true,
            "workspaceSymbolProvider": true,
            "callHierarchyProvider": true,
            "hoverProvider": true,
            "completionProvider": {"triggerCharacters": ["u", "i", "f"], "resolveProvider": true},
            "signatureHelpProvider": {"triggerCharacters": ["(", ","]},
            "renameProvider": {"prepareProvider": true},
            "codeActionProvider": {"codeActionKinds": ["source.format", "source.organizeImports"]},
            "codeLensProvider": {"resolveProvider": false},
            "semanticTokensProvider": {
                "full": true,
                "legend": {"tokenTypes": ["type", "number", "ownership-erg", "ownership-abs", "ownership-dat", "ownership-ins", "pack-keyword", "pack-name", "pack-field", "operator"], "tokenModifiers": ["inactive-target"]}
            },
            "actusSemanticModelProvider": true,
            "documentFormattingProvider": true,
            "documentRangeFormattingProvider": true
        },
        "serverInfo": { "name": "actus-lsp", "version": env!("CARGO_PKG_VERSION") },
        "actus": {
            "protocolVersion": ACTUS_LSP_PROTOCOL_VERSION,
            "schemaVersion": ACTUS_LSP_SCHEMA_VERSION,
            "target": target.triple().to_string(),
            "negotiatedCapabilities": negotiated,
            "compatibility": compatibility(params),
            "sourceWorkflowProvider": {"methods": ["actus/run"], "requiresDocumentVersion": true},
            "states": ["available", "stale", "partial", "unsupported", "invalid"]
        }
    })
}

fn compatibility(params: &Value) -> Value {
    let has_client_contract = params
        .get("initializationOptions")
        .and_then(|options| options.get("actus"))
        .and_then(|actus| actus.get("protocolVersion"))
        .is_some();
    json!({
        "mode": if has_client_contract { "versioned" } else { "legacy" },
        "serverProtocolVersion": ACTUS_LSP_PROTOCOL_VERSION,
        "serverSchemaVersion": ACTUS_LSP_SCHEMA_VERSION,
    })
}

pub(super) fn negotiated_capabilities(params: &Value) -> Vec<&'static str> {
    const SUPPORTED: &[&str] = &[
        "documentSync",
        "completion",
        "signatureHelp",
        "rename",
        "codeAction",
        "codeLens",
        "sourceWorkflow",
        "definition",
        "hover",
        "semanticTokens",
        "semanticModel",
        "formatting",
    ];
    let Some(requested) = params
        .get("initializationOptions")
        .and_then(|options| options.get("actus"))
        .and_then(|actus| actus.get("capabilities"))
        .and_then(Value::as_array)
    else {
        return SUPPORTED.to_vec();
    };
    requested
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|name| SUPPORTED.iter().copied().find(|supported| *supported == name))
        .collect()
}

pub(super) fn validate_initialize_contract(params: &Value) -> io::Result<()> {
    let Some(actus) = params.get("initializationOptions").and_then(|options| options.get("actus"))
    else {
        return Ok(());
    };
    validate_version(actus, "protocolVersion", ACTUS_LSP_PROTOCOL_VERSION)?;
    validate_version(actus, "schemaVersion", ACTUS_LSP_SCHEMA_VERSION)
}

fn validate_version(params: &Value, field: &str, expected: u32) -> io::Result<()> {
    let Some(value) = params.get(field) else { return Ok(()) };
    if value.as_u64() == Some(expected as u64) {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("unsupported Actus {field}; expected version {expected}"),
    ))
}

pub(super) fn response_metadata(
    target: &TargetSpec,
    params: &Value,
    store: &DocumentStore,
) -> ResponseMetadata {
    let document = params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
        .and_then(|uri| {
            store
                .get(uri)
                .map(|document| DocumentMetadata { uri: uri.to_owned(), version: document.version })
        });
    ResponseMetadata {
        protocol_version: ACTUS_LSP_PROTOCOL_VERSION,
        schema_version: ACTUS_LSP_SCHEMA_VERSION,
        compiler_version: env!("CARGO_PKG_VERSION"),
        target: target.triple().to_string(),
        result_state: "available",
        workspace_version: store.workspace_generation(),
        document,
    }
}
