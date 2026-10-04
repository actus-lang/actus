use std::collections::HashMap;
use std::path::PathBuf;

use serde_json::json;

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::position::{LineIndex, LspPosition};
use super::query_cache::ParseSnapshot;
use crate::ast::TopLevelDecl;
use crate::lexer::{SourceSpan, TokenKind, scan};
use crate::target::TargetSpec;

pub(super) struct Context<'a> {
    pub(super) target: &'a TargetSpec,
    pub(super) document_version: Option<i64>,
    pub(super) store: &'a DocumentStore,
    pub(super) cancellation: Option<&'a CancellationToken>,
}

pub(super) fn items(
    uri: &str,
    source: Option<&str>,
    overlays: &HashMap<PathBuf, String>,
    position: Option<&LspPosition>,
    context: Context<'_>,
) -> serde_json::Value {
    if context.cancellation.is_some_and(CancellationToken::checkpoint) {
        return serde_json::Value::Array(Vec::new());
    }
    let mut labels = builtin_labels();
    let mut semantic_items = Vec::new();
    labels.extend((1..=128).map(|width| format!("u{width}")));
    labels.extend((1..=128).map(|width| format!("i{width}")));
    if source.is_some()
        && let Some(ParseSnapshot::Valid(program)) = context.store.parse_snapshot(uri)
    {
        let program = crate::semantic::filter_program_for_target(&program, context.target);
        labels.extend(super::module_scope::internal_symbols(&program));
        labels.extend(super::module_scope::imported_public_symbols(uri, &program, overlays));
        semantic_items.extend(binding_items(&program));
        semantic_items.extend(pack_items(&program));
        for declaration in program.declarations {
            if let TopLevelDecl::Pack(pack) = declaration {
                labels.push(pack.name);
                labels.extend(pack.fields.into_iter().map(|field| field.name));
            }
        }
    }
    let replacement =
        source.and_then(|text| position.and_then(|value| replacement_range(text, value)));
    let mut items = labels
        .into_iter()
        .map(|label| {
            let mut item = json!({
                "label": label,
                "kind": completion_kind(&label),
                "detail": completion_detail(&label),
                "data": {"uri": uri, "symbol": label, "version": context.document_version},
            });
            if let Some(range) = replacement.clone() {
                item["textEdit"] = json!({"range": range, "newText": item["label"]});
            }
            item
        })
        .collect::<Vec<_>>();
    items.extend(semantic_items.into_iter().map(|mut item| {
        if let Some(range) = replacement.clone() {
            item["textEdit"] = json!({"range": range, "newText": item["label"]});
        }
        item
    }));
    serde_json::Value::Array(items)
}

fn completion_kind(label: &str) -> u8 {
    match label {
        "true" | "false" => 17,
        _ => 25,
    }
}

fn completion_detail(label: &str) -> &'static str {
    match label {
        "true" | "false" => "Actus Boolean literal",
        "Int" | "Bool" | "Char" | "String" | "Buffer" | "Array" | "Arena" | "Option" | "Result"
        | "Map" | "Usize" | "Void" | "f32" | "f64" => "Actus type",
        _ => "Actus symbol",
    }
}

fn pack_items(program: &crate::ast::Program) -> Vec<serde_json::Value> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Pack(pack) => Some(pack),
            _ => None,
        })
        .flat_map(|pack| {
            let storage = std::iter::once(json!({
                "label": pack.storage_name,
                "kind": 5,
                "detail": format!(
                    "pack {} storage: {} [{}]",
                    pack.name,
                    pack.storage.type_name().canonical_key(),
                    pack_storage_detail(&pack.storage),
                ),
                "data": {"kind": "pack-storage", "pack": pack.name, "symbol": pack.storage_name},
            }));
            let fields = pack.fields.iter().map(|field| {
                let width =
                    crate::ast::primitive_type(&field.ty.name).and_then(
                        |primitive| match primitive {
                            crate::ast::PrimitiveType::Integer { width, .. } => Some(width),
                            _ => None,
                        },
                    );
                let width_detail = width.map_or_else(
                    || "non-integer width".to_owned(),
                    |value| format!("{value} bits"),
                );
                json!({
                    "label": field.name,
                    "kind": 5,
                    "detail": format!(
                        "pack {} field: {} {}: {} [offset: {}, width: {}]",
                        pack.name, role_name(&field.role), field.name, field.ty.name,
                        field.offset, width_detail,
                    ),
                    "data": {"kind": "pack-field", "pack": pack.name, "symbol": field.name},
                })
            });
            storage.chain(fields)
        })
        .collect()
}

fn pack_storage_detail(storage: &crate::ast::PackStorage) -> String {
    match storage {
        crate::ast::PackStorage::ByteArray { capacity, element, .. } => {
            let bits =
                crate::ast::primitive_type(&element.name).and_then(|primitive| match primitive {
                    crate::ast::PrimitiveType::Integer { width, .. } => {
                        Some(u64::from(width).saturating_mul(*capacity))
                    }
                    _ => None,
                });
            bits.map_or_else(
                || format!("{capacity} bytes"),
                |value| format!("{capacity} bytes, {value} bits"),
            )
        }
        crate::ast::PackStorage::Scalar(type_name) => crate::ast::primitive_type(&type_name.name)
            .and_then(|primitive| match primitive {
                crate::ast::PrimitiveType::Integer { width, .. } => Some(format!("{width} bits")),
                _ => None,
            })
            .unwrap_or_else(|| "storage width unavailable".to_owned()),
    }
}

fn binding_items(program: &crate::ast::Program) -> Vec<serde_json::Value> {
    let Ok(model) = crate::semantic::analyze(program) else { return Vec::new() };
    model
        .bindings
        .iter()
        .enumerate()
        .map(|(index, binding)| {
            let type_name = model
                .binding_type_names
                .get(&index)
                .map(format_type_name)
                .or_else(|| binding.ty.map(|ty| ty.spec().name.to_owned()))
                .unwrap_or_else(|| "inferred".to_owned());
            json!({
                "label": binding.name,
                "kind": 6,
                "detail": format!(
                    "{} {}: {} [{}; {}]",
                    role_name(&binding.role), binding.name, type_name,
                    ownership_name(&binding.ownership), access_name(&binding.access),
                ),
                "data": {"kind": "binding", "symbol": binding.name},
            })
        })
        .collect()
}

fn format_type_name(type_name: &crate::ast::TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(format_type_name).collect::<Vec<_>>().join(", ")
    )
}

fn role_name(role: &crate::ast::Role) -> &'static str {
    match role {
        crate::ast::Role::Erg => "erg",
        crate::ast::Role::Abs => "abs",
        crate::ast::Role::Dat => "dat",
        crate::ast::Role::Ins => "ins",
    }
}

fn ownership_name(state: &crate::semantic::OwnershipState) -> &'static str {
    match state {
        crate::semantic::OwnershipState::Active => "Active",
        crate::semantic::OwnershipState::PartiallyMoved { .. } => "PartiallyMoved",
        crate::semantic::OwnershipState::Moved => "Moved",
        crate::semantic::OwnershipState::Dropped => "Dropped",
    }
}

fn access_name(state: &crate::semantic::AccessState) -> &'static str {
    match state {
        crate::semantic::AccessState::Mutable => "Mutable",
        crate::semantic::AccessState::Frozen { .. } => "Frozen",
        crate::semantic::AccessState::Suspended { .. } => "Suspended",
    }
}

pub(super) fn resolve_item(
    item: &mut serde_json::Value,
    store: &super::documents::DocumentStore,
    target: &TargetSpec,
) -> bool {
    let Some(data) = item.get("data") else { return false };
    let Some(uri) = data.get("uri").and_then(serde_json::Value::as_str) else { return false };
    let Some(symbol) = data.get("symbol").and_then(serde_json::Value::as_str) else {
        return false;
    };
    let Some(document) = store.get(uri) else { return true };
    if data
        .get("version")
        .and_then(serde_json::Value::as_i64)
        .is_some_and(|version| version != document.version)
    {
        return true;
    }
    let Some(span) = identifier_span(&document.text, symbol) else { return false };
    let position = LineIndex::new(&document.text).position(&document.text, span.start);
    if let Some(info) =
        super::hover::find_hover(uri, &document.text, &position, &store.source_overlays(), target)
    {
        item["documentation"] = json!({"kind": "markdown", "value": info.contents});
        item["detail"] = json!("Resolved Actus symbol");
        item["data"]["sourceRange"] = json!(info.range);
    }
    false
}

fn replacement_range(source: &str, position: &LspPosition) -> Option<serde_json::Value> {
    let offset = LineIndex::new(source).byte_offset(source, position)?;
    let start = scan(source)
        .0
        .into_iter()
        .filter_map(|token| {
            (token.span.start <= offset && offset <= token.span.end)
                .then_some(matches!(token.kind, TokenKind::Identifier(_)).then_some(token.span))
                .flatten()
        })
        .next()
        .map_or(offset, |span| span.start);
    let index = LineIndex::new(source);
    Some(json!({"start": index.position(source, start), "end": index.position(source, offset)}))
}

fn identifier_span(source: &str, name: &str) -> Option<SourceSpan> {
    scan(source).0.into_iter().find_map(|token| match token.kind {
        TokenKind::Identifier(value) if value == name => Some(token.span),
        _ => None,
    })
}

fn builtin_labels() -> Vec<String> {
    vec![
        "pack".to_owned(),
        "layout".to_owned(),
        "fields".to_owned(),
        "at".to_owned(),
        "little".to_owned(),
        "big".to_owned(),
        "f32".to_owned(),
        "f64".to_owned(),
        "Void".to_owned(),
        "Int".to_owned(),
        "Bool".to_owned(),
        "true".to_owned(),
        "false".to_owned(),
        "Char".to_owned(),
        "String".to_owned(),
        "Buffer".to_owned(),
        "Array".to_owned(),
        "Arena".to_owned(),
        "Option".to_owned(),
        "Result".to_owned(),
        "Map".to_owned(),
        "Usize".to_owned(),
        "erg".to_owned(),
        "abs".to_owned(),
        "dat".to_owned(),
        "ins".to_owned(),
        "copy".to_owned(),
        "as".to_owned(),
        "<".to_owned(),
        "<=".to_owned(),
        ">".to_owned(),
        ">=".to_owned(),
        "==".to_owned(),
        "!=".to_owned(),
        "%".to_owned(),
        "&&".to_owned(),
        "||".to_owned(),
        "&".to_owned(),
        "|".to_owned(),
        "^".to_owned(),
        "~".to_owned(),
        "<<".to_owned(),
        ">>".to_owned(),
    ]
}
