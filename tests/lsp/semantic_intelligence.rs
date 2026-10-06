use serde_json::{Value, json};

use crate::lsp_support::{position_after, response_with_id, run_lsp};

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

#[test]
fn semantic_model_exposes_serialization_contract_sections() {
    let uri = "file:///tmp/actus-lsp-serialization-contract.act";
    let source = concat!(
        "pack FramePack { erg storage: Array[u8, 8]; layout little; fields { ",
        "erg byte_0: u8 at 0; erg byte_1: u8 at 8; erg byte_2: u8 at 16; erg byte_3: u8 at 24; ",
        "erg byte_4: u8 at 32; erg byte_5: u8 at 40; erg byte_6: u8 at 48; erg byte_7: u8 at 56; } }\n",
        "serialize Frame from FramePack { layout little; version u16 at 0; payload bytes at 2 length 1; ",
        "checksum crc32 over 0 .. 3 at 3; }\n",
        "verb main() -> Int { return 0; }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let model = response_with_id(&stdout, 2);
    let contract = &model["result"]["serializations"][0];
    assert_eq!(contract["name"], "Frame");
    assert_eq!(contract["sourceType"], "FramePack");
    assert_eq!(contract["endianness"], "little");
    assert_eq!(contract["sections"][0]["kind"], "version");
    assert_eq!(contract["sections"][0]["offset"], 0);
    assert_eq!(contract["sections"][1]["kind"], "payload");
    assert_eq!(contract["sections"][1]["length"], 1);
    assert_eq!(contract["sections"][2]["kind"], "checksum");
    assert_eq!(contract["sections"][2]["offset"], 3);
}

#[test]
fn semantic_model_exposes_structured_verb_contract_sections() {
    let uri = "file:///tmp/actus-lsp-verb-contract.act";
    let source = concat!(
        "\"\"\"\n",
        "contract:\n",
        "purpose:\n",
        "    Read one frame.\n",
        "ownership:\n",
        "    The caller keeps ownership.\n",
        "errors:\n",
        "    Returns a typed error.\n",
        "\"\"\"\n",
        "open verb read_frame(abs source: Buffer) -> Int { return 0; }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source,"read_frame")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let model = response_with_id(&stdout, 2);
    let declaration = model["result"]["declarations"]
        .as_array()
        .expect("declarations")
        .iter()
        .find(|declaration| declaration["name"] == "read_frame")
        .expect("read_frame declaration");
    assert_eq!(declaration["contract"]["sections"][0]["kind"], "purpose");
    assert_eq!(declaration["contract"]["sections"][0]["text"], "Read one frame.");
    assert_eq!(declaration["contract"]["sections"][1]["kind"], "ownership");
    assert_eq!(declaration["contract"]["sections"][2]["kind"], "errors");
    assert!(stdout.contains("**Contract**"));
    assert!(stdout.contains("Read one frame."));
}

#[test]
fn semantic_model_and_hover_expose_inferred_argument_roles() {
    let uri = "file:///tmp/actus-lsp-inferred-roles.act";
    let source = concat!(
        "verb read(abs value: Int) -> Int { return 0; }\n",
        "verb update(ins value: Int) { value += 1; }\n",
        "verb main() -> Int { erg source = 1; abs view = ref source; ",
        "read(value: view); ins mutable = 0; update(value: mutable); return 0; }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source,"read(value: view")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let model = response_with_id(&stdout, 2);
    let roles = model["result"]["argumentRoles"].as_array().expect("argument roles");
    assert!(roles.iter().any(|fact| fact["callee"] == "read"
        && fact["role"] == "abs"
        && fact["source"] == "inferred"));
    assert!(roles.iter().any(|fact| fact["callee"] == "update"
        && fact["role"] == "ins"
        && fact["source"] == "inferred"));
    assert!(stdout.contains("argument role: abs (inferred)"), "hover missing: {stdout}");
}

#[test]
fn semantic_model_hides_generated_scalar_locals() {
    let uri = "file:///tmp/actus-lsp-generated-scalar-local.act";
    let source = concat!(
        "verb read(abs value: u32) -> Int { return value as Int; }\n",
        "verb main() -> Int { erg index: u32 = 41u32; ",
        "return read(value: abs (index + 1u32)); }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let model = response_with_id(&run_lsp(messages.to_vec()), 2);
    assert_eq!(model["result"]["state"], "available");
    let bindings = model["result"]["bindings"].as_array().expect("bindings");
    assert!(bindings.iter().all(|binding| {
        !binding["name"].as_str().is_some_and(|name| name.starts_with("__actus_generated_"))
    }));
    assert!(bindings.iter().any(|binding| binding["name"] == "index"));
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
fn semantic_model_exposes_limitless_scope_for_declarations() {
    let uri = "file:///tmp/actus-lsp-limitless.act";
    let source = concat!("meta limitless(\"verb\")\n", "verb verb_scope() -> Int { return 2; }\n",);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let model = response_with_id(&run_lsp(messages.to_vec()), 2);
    assert_eq!(model["result"]["declarations"][0]["limitless"], "verb");
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

#[test]
fn lsp_exposes_array_pack_layout_facts_and_reindexes_overlay_changes() {
    let uri = "file:///tmp/actus-lsp-array-pack.act";
    let source = concat!(
        "pack Frame { erg storage: Array[u8, 4]; layout little; fields { ",
        "erg marker: u8 at 0; abs _reserved: u24 at 8 = 0; } }\n",
        "verb main() -> Int { erg frame = Frame { storage: Array[u8, 4](), }; ",
        "return frame.storage[0] as Int; }\n",
    );
    let updated = source.replace("layout little", "layout big");
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":5}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{"textDocument":{"uri":uri},"position":{"line":0,"character":0}}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":updated}]}}),
        json!({"jsonrpc":"2.0","id":5,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":2}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let initial = response_with_id(&stdout, 2);
    assert_eq!(initial["result"]["packs"][0]["storageBits"], 32);
    assert_eq!(initial["result"]["packs"][0]["storageBytes"], 4);
    assert_eq!(initial["result"]["packs"][0]["storageCapacity"], 4);
    assert_eq!(initial["result"]["packs"][0]["layout"], "little");
    assert!(stdout.contains("storage: 4 bytes, 32 bits"));
    assert!(stdout.contains("\"label\":\"storage\""));
    let changed = response_with_id(&stdout, 5);
    assert_eq!(changed["result"]["packs"][0]["layout"], "big");
}

#[test]
fn lsp_exposes_indexed_pack_field_facts_and_hover() {
    let uri = "file:///tmp/actus-lsp-indexed-pack-field.act";
    let source = concat!(
        "pack Example { erg storage: Array[u8, 32]; layout little; fields { ",
        "erg links: Array[u32, 8] at 0; } }\n",
        "verb main() -> Int { return 0; }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/hover","params":{"textDocument":{"uri":uri},"position":position_after(source, "links")}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let model = response_with_id(&stdout, 2);
    let field = &model["result"]["packs"][0]["fields"][0];
    assert_eq!(field["type"], "Array[u32, 8]");
    assert_eq!(field["elementType"], "u32");
    assert_eq!(field["count"], 8);
    assert_eq!(field["width"], 256);
    assert!(stdout.contains("element: u32, count: 8"), "stdout: {stdout}");
}

#[test]
fn lsp_accepts_pack_types_as_array_element_types() {
    let uri = "file:///tmp/actus-lsp-pack-array.act";
    let source = concat!(
        "pack Cell { erg storage: u8; layout little; fields { erg marker: u8 at 0; } }\n",
        "struct Fabric { cells: Array[Cell, 2], }\n",
        "verb main() -> Int { erg cells: Array[Cell, 2] = Array[Cell, 2](); return 0; }\n",
    );
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"actus/semanticModel","params":{"textDocument":{"uri":uri,"version":1}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let model = response_with_id(&run_lsp(messages.to_vec()), 2);
    assert_eq!(model["result"]["state"], "available");
    assert_eq!(model["result"]["packs"][0]["name"], "Cell");
}

#[test]
fn lsp_navigates_and_renames_array_pack_storage() {
    let uri = "file:///tmp/actus-lsp-pack-storage-navigation.act";
    let source = "pack Frame { erg storage: Array[u8, 4]; layout little; fields { erg marker: u8 at 0; abs _reserved: u24 at 8 = 0; } }\nverb main() -> Int { erg frame = Frame { storage: Array[u8, 4](), }; return frame.storage[0] as Int; }\n";
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":source}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{"textDocument":{"uri":uri},"position":position_after(source,"frame.storage")}}),
        json!({"jsonrpc":"2.0","id":3,"method":"textDocument/rename","params":{"textDocument":{"uri":uri},"position":position_after(source,"frame.storage"),"newName":"bytes"}}),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ];
    let stdout = run_lsp(messages.to_vec());
    let definition = response_with_id(&stdout, 2);
    let rename = response_with_id(&stdout, 3);
    let definition_text = definition.to_string();
    assert!(
        definition_text.contains("\"uri\":\"file:///tmp/actus-lsp-pack-storage-navigation.act\""),
        "storage definition missing: {definition_text}"
    );
    assert!(
        definition_text.contains("\"character\":17"),
        "storage declaration span missing: {definition_text}"
    );
    let edits = rename["result"]["changes"][uri].as_array().expect("storage rename edits");
    assert_eq!(edits.len(), 3, "storage declaration and both uses should be renamed: {rename}");
    assert!(
        rename.to_string().contains("\"newText\":\"bytes\""),
        "storage rename text missing: {rename}"
    );
}
