use serde_json::json;

use crate::lsp_support::{fixture_messages, position_after, run_lsp};

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
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("markdown"));
    assert!(stdout.contains("abs value: Int"));
    assert!(stdout.contains("Adds a value."));
    assert!(stdout.contains("The value is inspected without ownership transfer."));
    assert!(stdout.contains("newText"));
    assert!(stdout.contains("verb add(abs value: Int) -> Int"));
    assert!(stdout.contains("\"compilerVersion\":\"0.1.0\""), "stdout: {stdout}");
    assert!(stdout.contains("\"target\":"), "stdout: {stdout}");
    assert!(
        stdout.contains("\"document\":{\"uri\":\"file:///tmp/actus-lsp-hover.act\",\"version\":1}"),
        "stdout: {stdout}"
    );
}

#[test]
fn lsp_signature_help_exposes_roles_types_and_active_parameter() {
    let uri = "file:///tmp/actus-lsp-signature-help.act";
    let source = "verb combine(abs left: Int, ins right: Buffer) -> Int { return left; }\nverb caller() -> Int { return combine(left: 1, right: Buffer[0]); }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":uri},"position":position_after(source, "left: 1,")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("signatureHelpProvider"), "stdout: {stdout}");
    assert!(stdout.contains("abs left: Int"), "stdout: {stdout}");
    assert!(stdout.contains("ins right: Buffer"), "stdout: {stdout}");
    assert!(stdout.contains("\"activeParameter\":1"), "stdout: {stdout}");
}

#[test]
fn lsp_signature_help_exposes_generics_and_result_contract() {
    let uri = "file:///tmp/actus-lsp-generic-signature.act";
    let source = "verb convert[T: Reader](abs input: T) -> Result[Buffer, IoError] { return Result.Ok(input); }\nverb main() -> Void { convert(input: input); }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":uri},"position":position_after(source, "convert(input")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("convert[T: Reader]("), "stdout: {stdout}");
    assert!(stdout.contains("Result[Buffer, IoError]"), "stdout: {stdout}");
}

#[test]
fn lsp_signature_help_rejects_stale_document_versions() {
    let uri = "file:///tmp/actus-lsp-signature-stale.act";
    let source = "verb add(abs value: Int) -> Int { return value; }\nverb main() -> Int { return add(value: 1); }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":2,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp","params":{"textDocument":{"uri":uri,"version":1},"position":position_after(source, "add(value")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"resultState\":\"stale\""), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":2"), "stdout: {stdout}");
    assert!(stdout.contains("\"result\":null"), "stdout: {stdout}");
}

#[test]
fn lsp_completion_items_support_context_edits_and_resolution() {
    let uri = "file:///tmp/actus-lsp-completion-resolution.act";
    let source = "/// Adds a value from the current module.\nverb add(abs value: Int) -> Int { return value; }\nverb main() -> Int { return add(value: 1); }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":30}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"completionItem/resolve","params":{"label":"add","data":{"uri":uri,"symbol":"add"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("resolveProvider"), "stdout: {stdout}");
    assert!(stdout.contains("\"textEdit\""), "stdout: {stdout}");
    assert!(stdout.contains("Adds a value from the current module."), "stdout: {stdout}");
}

#[test]
fn lsp_completion_resolution_rejects_a_stale_document_snapshot() {
    let uri = "file:///tmp/actus-lsp-completion-stale.act";
    let source = "/// Adds a value.\nverb add(abs value: Int) -> Int { return value; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":2,"character":0}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":source}]}}),
        json!({"jsonrpc":"2.0","id":3,"method":"completionItem/resolve","params":{"label":"add","data":{"uri":uri,"symbol":"add","version":1}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"resultState\":\"stale\""), "stdout: {stdout}");
}

#[test]
fn lsp_completion_marks_incomplete_source_as_partial_without_hiding_intrinsics() {
    let uri = "file:///tmp/actus-lsp-incomplete-completion.act";
    let source = "verb broken(abs value: Int) -> Int { return value\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":45}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"resultState\":\"partial\""), "stdout: {stdout}");
    assert!(stdout.contains("\"label\":\"Buffer\""), "intrinsic completion missing: {stdout}");
}

#[test]
fn lsp_initialize_exposes_versioned_contract_and_rejects_invalid_params() {
    let messages =
        fixture_messages(include_str!("../fixtures/lsp/protocol/initialize_contract.json"));
    let stdout = run_lsp(messages);
    assert!(stdout.contains("\"protocolVersion\":1"), "stdout: {stdout}");
    assert!(stdout.contains("\"schemaVersion\":1"), "stdout: {stdout}");
    assert!(stdout.contains("\"mode\":\"legacy\""), "stdout: {stdout}");
    assert!(
        stdout.contains(
            "\"states\":[\"available\",\"stale\",\"partial\",\"unsupported\",\"invalid\"]"
        ),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("\"id\":2"), "stdout: {stdout}");
    assert!(stdout.contains("\"code\":-32602"), "stdout: {stdout}");
    assert!(stdout.contains("\"resultState\":\"invalid\""), "stdout: {stdout}");
}

#[test]
fn lsp_rejects_incompatible_contracts_and_unknown_requests_structurally() {
    let messages =
        fixture_messages(include_str!("../fixtures/lsp/protocol/incompatible_requests.json"));
    let stdout = run_lsp(messages);
    assert!(stdout.contains("unsupported Actus protocolVersion"), "stdout: {stdout}");
    assert!(stdout.contains("\"code\":-32601"), "stdout: {stdout}");
    assert!(stdout.contains("\"resultState\":\"unsupported\""), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":3"), "stdout: {stdout}");
}

#[test]
fn lsp_negotiates_supported_capabilities_and_enforces_lifecycle() {
    let messages =
        fixture_messages(include_str!("../fixtures/lsp/protocol/lifecycle_and_capabilities.json"));
    let stdout = run_lsp(messages);
    assert!(stdout.contains("\"negotiatedCapabilities\":[\"hover\"]"), "stdout: {stdout}");
    assert!(stdout.contains("\"mode\":\"versioned\""), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":2"), "stdout: {stdout}");
    assert!(stdout.contains("\"id\":4"), "stdout: {stdout}");
    assert!(stdout.matches("\"code\":-32600").count() >= 2, "stdout: {stdout}");
}

#[test]
fn lsp_cancels_queued_requests_without_publishing_results() {
    let messages =
        fixture_messages(include_str!("../fixtures/lsp/protocol/canceled_exchange.json"));
    let stdout = run_lsp(messages);
    assert!(stdout.contains("\"code\":-32800"), "stdout: {stdout}");
    assert!(stdout.contains("\"resultState\":\"partial\""), "stdout: {stdout}");
    assert!(!stdout.contains("\"id\":2,\"result\""), "stdout: {stdout}");
}

#[test]
fn lsp_emits_bounded_query_progress_notifications() {
    let uri = "file:///tmp/actus-lsp-progress.act";
    let source = "verb main() -> Int { return 1; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}
        }),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"method\":\"$/progress\""), "stdout: {stdout}");
    assert!(stdout.contains("\"kind\":\"begin\""), "stdout: {stdout}");
    assert!(stdout.contains("\"kind\":\"report\""), "stdout: {stdout}");
    assert!(stdout.contains("\"kind\":\"end\""), "stdout: {stdout}");
}

#[test]
fn lsp_rejects_stale_document_updates_without_replacing_the_overlay() {
    let uri = "file:///tmp/actus-lsp-stale.act";
    let stable_source = "verb stable() -> Int { return 7; }\n";
    let stale_source = "verb replaced() -> Int { return missing; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":2,"text":stable_source}}
        }),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didChange",
            "params":{"textDocument":{"uri":uri,"version":1},"contentChanges":[{"text":stale_source}]}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}
        }),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("stable"), "stdout: {stdout}");
    assert!(!stdout.contains("replaced"), "stdout: {stdout}");
    assert!(stdout.contains("\"version\":2"), "stdout: {stdout}");
}

#[test]
fn lsp_marks_large_completion_results_partial_and_preserves_tail_symbols() {
    let uri = "file:///tmp/actus-lsp-bounded-completion.act";
    let source = (0..600)
        .map(|index| format!("verb symbol_{index}() -> Int {{ return {index}; }}\n"))
        .collect::<String>();
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":1,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/completion",
            "params":{"textDocument":{"uri":uri}}
        }),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"resultState\":\"partial\""), "stdout: {stdout}");
    assert!(stdout.contains("symbol_599"), "stdout: {stdout}");
}

#[test]
fn lsp_marks_version_pinned_queries_stale_without_returning_old_data() {
    let uri = "file:///tmp/actus-lsp-query-stale.act";
    let source = "verb stable() -> Int { return 7; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":uri,"version":2,"text":source}}
        }),
        json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":uri,"version":1},"position":{"line":0,"character":5}}
        }),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("\"resultState\":\"stale\""), "stdout: {stdout}");
    assert!(!stdout.contains("verb stable"), "stdout: {stdout}");
}
