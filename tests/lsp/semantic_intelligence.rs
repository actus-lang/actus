use serde_json::{Value, json};

use crate::lsp_support::{response_with_id, run_lsp};

#[test]
fn semantic_model_exposes_compiler_owned_resource_and_target_facts() {
    let uri = "file:///tmp/actus-lsp-semantic-model.act";
    let source = concat!(
        "meta target(\"unix\")\n",
        "verb unix_only() -> Int { return 1; }\n",
        "meta target(\"windows\")\n",
        "verb windows_only() -> Int { return 2; }\n",
        "pack Register { erg storage: u8; layout little; fields { erg low: u4 at 0; abs high: u4 at 4; } }\n",
        "verb main() -> Int { erg bytes = Buffer[4]; erg values: Array[u8, 2] = Array[u8, 2](); erg value: f32 = 1.5f32; return if value == 1.5f32 { 41 } else { 0 }; }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"initializationOptions":{"target":"x86_64-unknown-linux-gnu"}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":2,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":2}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let model = response_with_id(&stdout, 2);
    assert_eq!(model["result"]["state"], "available");
    assert_eq!(model["result"]["bindings"][0]["indexed"]["elementType"], "u8");
    assert_eq!(model["result"]["bindings"][1]["indexed"]["capacity"], "2");
    assert_eq!(model["result"]["packs"][0]["storageBits"], 8);
    assert_eq!(model["result"]["packs"][0]["fields"][0]["offset"], 0);
    assert_eq!(model["result"]["literals"][0]["kind"], "integer");
    assert!(model["result"]["literals"].as_array().is_some_and(|facts| {
        facts.iter().any(|fact| fact["suffix"] == "f32" && fact["type"] == "f32")
    }));
    assert!(model["result"]["conditionals"].as_array().is_some_and(|facts| {
        facts.iter().any(|fact| {
            fact["condition"]["type"] == "Bool"
                && fact["thenType"] == "Int"
                && fact["elseType"] == "Int"
        })
    }));
    assert!(model["result"]["target"]["triple"].as_str().is_some());
    assert!(active_declaration(&model, "unix_only"));
    assert!(!active_declaration(&model, "windows_only"));
}

fn active_declaration(model: &Value, name: &str) -> bool {
    model["result"]["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|declaration| declaration["name"] == name)
        .is_some_and(|declaration| declaration["active"] == true)
}

#[test]
fn hover_explains_index_bounds_and_pack_data_model() {
    let uri = "file:///tmp/actus-lsp-semantic-hover.act";
    let source = "pack Register { erg storage: u8; layout little; fields { erg low: u4 at 0; abs high: u4 at 4; } }\nverb main() -> Int { erg bytes = Buffer[4]; return 0; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":1,"character":26}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    assert!(stdout.contains("0 <= index < length"), "buffer bounds missing: {stdout}");
    assert!(stdout.contains("pointer width"), "target data model missing: {stdout}");
}
