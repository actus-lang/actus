use serde_json::{Value, json};

use crate::lsp_support::{response_with_id, run_lsp};

#[test]
fn memory_graph_exposes_compiler_owned_states_and_edges() {
    let uri = "file:///tmp/actus-lsp-memory-graph.act";
    let source = "/// Mutates a buffer.\nverb mutate(ins buffer: Buffer) { }\nverb slice(abs input: Buffer) -> abs Buffer { return input; }\nunsafe extern \"C\" verb host(erg code: Int) -> Int;\nverb main(abs ready: Bool) -> Int { erg buffer = Buffer[4]; erg code = 0; mutate(buffer: ins buffer); abs view = ref slice(input: abs buffer); return case abs ready { true if ready => host(code: erg code), _ => 1, }; }\n";
    let output = run_lsp(vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"initializationOptions":{"actus":{"capabilities":["memoryGraph"]}}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":4,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/memoryGraph","params":{"textDocument":{"uri":uri,"version":4}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ]);
    let response = response_with_id(&output, 2);
    assert_eq!(response["result"]["state"], "available");
    assert_eq!(response["result"]["schemaVersion"], 1);
    assert!(response["result"]["complete"].as_bool().unwrap_or(false));
    assert!(has_kind(&response["result"]["nodes"], "binding"));
    assert!(has_kind(&response["result"]["nodes"], "view"));
    assert!(has_kind(&response["result"]["nodes"], "loan"));
    assert!(has_kind(&response["result"]["edges"], "viewOrigin"));
    assert!(has_kind(&response["result"]["edges"], "loan"));
}

#[test]
fn memory_graph_rejects_stale_document_versions_without_guessing() {
    let uri = "file:///tmp/actus-lsp-memory-graph-stale.act";
    let output = run_lsp(vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":5,"text":"verb main() -> Int { return 0; }\n"}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/memoryGraph","params":{"textDocument":{"uri":uri,"version":4}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ]);
    let response = response_with_id(&output, 2);
    assert!(response["result"].is_null());
    assert_eq!(response["actus"]["resultState"], "stale");
}

fn has_kind(values: &Value, kind: &str) -> bool {
    values.as_array().into_iter().flatten().any(|entry| entry["kind"] == kind)
}
