use std::io::{self, Write};

use serde_json::{Value, json};

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::protocol::ResponseMetadata;
use super::query_handlers::stale_version;
use crate::target::TargetSpec;

pub(super) fn dispatch(
    id: Option<Value>,
    params: Value,
    store: &DocumentStore,
    target: &TargetSpec,
    output: &mut impl Write,
    mut metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    let Some((uri, version)) = requested_document(&params) else {
        return unavailable(id, output, &mut metadata, cancellation, "missing document identity");
    };
    let Some(document) = store.get(uri) else {
        return unavailable(id, output, &mut metadata, cancellation, "document is not open");
    };
    if stale_version(document.version, version) {
        metadata.result_state = "stale";
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    }
    if cancellation.is_some_and(CancellationToken::checkpoint) {
        return unavailable(id, output, &mut metadata, cancellation, "request canceled");
    }
    let semantic = super::semantic_model::query(uri, &document.text, store, target, cancellation);
    let graph = build_graph(uri, document.version, semantic);
    metadata.result_state = match graph["state"].as_str() {
        Some("available") => "available",
        Some("partial") => "partial",
        Some("invalid") => "invalid",
        _ => "unsupported",
    };
    super::server::respond(output, id, graph, metadata, cancellation)
}

fn requested_document(params: &Value) -> Option<(&str, Option<i64>)> {
    let document = params.get("textDocument")?;
    Some((document.get("uri")?.as_str()?, document.get("version").and_then(Value::as_i64)))
}

fn unavailable(
    id: Option<Value>,
    output: &mut impl Write,
    metadata: &mut ResponseMetadata,
    cancellation: Option<&CancellationToken>,
    reason: &str,
) -> io::Result<()> {
    metadata.result_state = "unsupported";
    super::server::respond(
        output,
        id,
        json!({"schemaVersion": 1, "state": "unavailable", "complete": false, "reason": reason, "nodes": [], "edges": [], "transitions": []}),
        metadata.clone(),
        cancellation,
    )
}

fn build_graph(uri: &str, version: i64, semantic: Value) -> Value {
    if semantic["state"] != "available" {
        return json!({
            "schemaVersion": 1,
            "state": semantic["state"],
            "complete": false,
            "reason": semantic["reason"].as_str().unwrap_or("semantic model unavailable"),
            "nodes": [], "edges": [], "transitions": [],
        });
    }
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut transitions = Vec::new();
    add_bindings(&semantic, &mut nodes, &mut transitions);
    add_borrows(&semantic, &mut nodes, &mut edges);
    add_loans(&semantic, &mut nodes, &mut edges);
    add_cleanup(&semantic, &mut nodes, &mut edges);
    add_declarations(&semantic, &mut nodes);
    json!({
        "schemaVersion": 1,
        "state": "available",
        "complete": true,
        "document": {"uri": uri, "version": version},
        "nodes": nodes,
        "edges": edges,
        "transitions": transitions,
        "sourceEncoding": "utf-8-to-utf-16",
    })
}

fn add_bindings(semantic: &Value, nodes: &mut Vec<Value>, transitions: &mut Vec<Value>) {
    for binding in semantic["bindings"].as_array().into_iter().flatten() {
        let id = format!("binding:{}", binding["index"]);
        nodes.push(json!({"id": id, "kind": "binding", "name": binding["name"], "role": binding["role"], "state": binding["ownership"], "range": binding["range"]}));
        transitions.push(json!({"node": id, "from": Value::Null, "to": binding["ownership"], "range": binding["range"], "source": "semantic-analysis"}));
    }
}

fn add_borrows(semantic: &Value, nodes: &mut Vec<Value>, edges: &mut Vec<Value>) {
    for borrow in semantic["borrows"].as_array().into_iter().flatten() {
        let id = format!("borrow:{}", borrow["id"]);
        nodes.push(
            json!({"id": id, "kind": "view", "state": "Frozen", "range": borrow["originRange"]}),
        );
        edges.push(json!({"kind": "viewOrigin", "from": format!("binding:{}", borrow["owner"]), "to": id, "range": borrow["originRange"]}));
    }
}

fn add_loans(semantic: &Value, nodes: &mut Vec<Value>, edges: &mut Vec<Value>) {
    for loan in semantic["loans"].as_array().into_iter().flatten() {
        let id = format!("loan:{}", loan["id"]);
        nodes.push(json!({"id": id, "kind": "loan", "state": "Suspended", "callee": loan["callee"], "parameter": loan["parameter"], "range": loan["originRange"]}));
        edges.push(json!({"kind": "loan", "from": format!("binding:{}", loan["owner"]), "to": id, "range": loan["originRange"]}));
    }
}

fn add_cleanup(semantic: &Value, nodes: &mut Vec<Value>, edges: &mut Vec<Value>) {
    for (plan_index, plan) in semantic["cleanup"].as_array().into_iter().flatten().enumerate() {
        let id = format!("cleanup:{}", plan_index);
        nodes.push(json!({"id": id, "kind": "cleanup", "state": "Planned", "range": plan["range"], "actions": plan["actions"]}));
        edges.push(json!({"kind": "cleanup", "from": id, "to": format!("scope:{}", plan["depth"]), "range": plan["range"]}));
    }
}

fn add_declarations(semantic: &Value, nodes: &mut Vec<Value>) {
    for declaration in semantic["declarations"].as_array().into_iter().flatten() {
        nodes.push(json!({"id": format!("declaration:{}", declaration["name"]), "kind": "declaration", "name": declaration["name"], "active": declaration["active"], "range": declaration["range"]}));
    }
}
