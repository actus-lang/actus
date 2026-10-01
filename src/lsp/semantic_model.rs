use serde_json::{Value, json};

use crate::ast::{MetaAttribute, Program, TopLevelDecl, TypeName, primitive_type};
use crate::lexer::SourceSpan;
use crate::semantic::{
    AccessState, CleanupAction, OwnershipState, SemanticModel, analyze, filter_program_for_target,
};
use crate::target::{EntryContract, TargetSpec};

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::position::{LineIndex, LspRange};
use super::query_cache::ParseSnapshot;

/// Builds the compiler-owned semantic snapshot exposed by `actus/semanticModel`.
pub(super) fn query(
    uri: &str,
    source: &str,
    store: &DocumentStore,
    target: &TargetSpec,
    cancellation: Option<&CancellationToken>,
) -> Value {
    if cancellation.is_some_and(CancellationToken::checkpoint) {
        return json!({"state":"partial","reason":"canceled","facts":[]});
    }
    let version = store.get(uri).map(|document| document.version).unwrap_or_default();
    let target_name = target.triple().to_string();
    if let Some(result) = store.cached_semantic(uri, &target_name, version) {
        return result;
    }
    let Some(ParseSnapshot::Valid(program)) = store.parse_snapshot(uri) else {
        return json!({"state":"invalid","facts":[]});
    };
    if cancellation.is_some_and(CancellationToken::checkpoint) {
        return json!({"state":"partial","reason":"canceled","facts":[]});
    }
    let target_program = filter_program_for_target(&program, target);
    let Ok(model) = analyze(&target_program) else { return json!({"state":"invalid","facts":[]}) };
    if cancellation.is_some_and(CancellationToken::checkpoint) {
        return json!({"state":"partial","reason":"canceled","facts":[]});
    }
    let index = LineIndex::new(source);
    let result = json!({
        "state": "available",
        "document": {"uri": uri, "version": store.get(uri).map(|document| document.version)},
        "target": target_capabilities(target),
        "bindings": bindings(source, &index, &model),
        "borrows": borrows(source, &index, &model),
        "loans": loans(source, &index, &model),
        "cleanup": cleanup(source, &index, &model),
        "literals": literals(source, &index, &model),
        "conditionals": conditionals(source, &index, &model),
        "packs": packs(source, &index, &target_program),
        "declarations": declarations(source, &index, &program, target),
    });
    store.cache_semantic(uri, &target_name, version, result.clone());
    result
}

fn literals(source: &str, index: &LineIndex, model: &SemanticModel) -> Vec<Value> {
    model
        .literal_facts
        .iter()
        .map(|fact| {
            json!({
                "kind": fact.kind,
                "value": fact.value,
                "suffix": fact.suffix,
                "type": fact.type_name,
                "range": span_range(source, index, fact.span),
            })
        })
        .collect()
}

fn conditionals(source: &str, index: &LineIndex, model: &SemanticModel) -> Vec<Value> {
    model
        .conditional_facts
        .iter()
        .map(|fact| {
            json!({
                "range": span_range(source, index, fact.span),
                "condition": {
                    "type": fact.condition_type,
                    "range": span_range(source, index, fact.condition_span),
                },
                "thenType": fact.then_type,
                "elseType": fact.else_type,
            })
        })
        .collect()
}

fn bindings(source: &str, index: &LineIndex, model: &SemanticModel) -> Vec<Value> {
    model
        .bindings
        .iter()
        .enumerate()
        .map(|(binding_index, binding)| {
            let type_name = model
                .binding_type_names
                .get(&binding_index)
                .map(format_type)
                .or_else(|| binding.ty.map(|ty| ty.spec().name.to_owned()))
                .unwrap_or_else(|| "inferred".to_owned());
            let mut result = json!({
                "index": binding_index,
                "name": binding.name,
                "role": role_name(&binding.role),
                "type": type_name,
                "ownership": ownership_name(&binding.ownership),
                "access": access_name(&binding.access),
                "range": source_range(source, index, binding.span),
            });
            if let Some(details) = indexed_type_details(&type_name) {
                result["indexed"] = details;
            }
            result
        })
        .collect()
}

fn indexed_type_details(type_name: &str) -> Option<Value> {
    if type_name == "Buffer" {
        return Some(json!({
            "elementType": "u8",
            "bounds": "0 <= index < length",
            "outOfBounds": "deterministic runtime trap",
        }));
    }
    let arguments =
        type_name.strip_prefix("Array[")?.strip_suffix(']')?.split(", ").collect::<Vec<_>>();
    (arguments.len() == 2).then(|| {
        json!({
            "elementType": arguments[0],
            "capacity": arguments[1],
            "bounds": format!("0 <= index < {}", arguments[1]),
            "outOfBounds": "deterministic runtime trap",
        })
    })
}

fn borrows(source: &str, index: &LineIndex, model: &SemanticModel) -> Vec<Value> {
    model
        .borrows
        .iter()
        .map(|borrow| {
            json!({
                "id": borrow.id,
                "owner": borrow.owner,
                "field": borrow.field,
                "scopeDepth": borrow.scope_depth,
                "originRange": span_range(source, index, borrow.origin_span),
            })
        })
        .collect()
}

fn loans(source: &str, index: &LineIndex, model: &SemanticModel) -> Vec<Value> {
    model
        .exclusive_loans
        .iter()
        .map(|loan| {
            json!({
                "id": loan.id,
                "owner": loan.owner,
                "callee": loan.callee,
                "parameter": loan.parameter,
                "originRange": span_range(source, index, loan.origin_span),
            })
        })
        .collect()
}

fn cleanup(source: &str, index: &LineIndex, model: &SemanticModel) -> Vec<Value> {
    model
        .cleanup_plans
        .iter()
        .map(|plan| {
            json!({
                "depth": plan.depth,
                "range": span_range(source, index, plan.span),
                "actions": plan.actions.iter().map(cleanup_action).collect::<Vec<_>>(),
            })
        })
        .collect()
}

fn cleanup_action(action: &CleanupAction) -> Value {
    match action {
        CleanupAction::EndBorrow { borrow_id } => json!({"kind":"endBorrow","borrowId":borrow_id}),
        CleanupAction::DropBinding { binding_index } => {
            json!({"kind":"dropBinding","bindingIndex":binding_index})
        }
        CleanupAction::ResetArena { binding_index } => {
            json!({"kind":"resetArena","bindingIndex":binding_index})
        }
        CleanupAction::DropPayloadField { binding_index, enum_name, variant, field } => json!({
            "kind":"dropPayloadField", "bindingIndex":binding_index,
            "enum":enum_name, "variant":variant, "field":field,
        }),
    }
}

fn packs(source: &str, index: &LineIndex, program: &Program) -> Vec<Value> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Pack(pack) = declaration else { return None };
            let storage_bits = primitive_type(&pack.storage.name).and_then(|primitive| match primitive {
                crate::ast::PrimitiveType::Integer { width, .. } => Some(width),
                _ => None,
            });
            Some(json!({
                "name": pack.name,
                "storage": format_type(&pack.storage),
                "storageBits": storage_bits,
                "layout": format!("{:?}", pack.endianness).to_lowercase(),
                "range": span_range(source, index, pack.span),
                "fields": pack.fields.iter().map(|field| {
                    let width = primitive_type(&field.ty.name).and_then(|primitive| match primitive {
                        crate::ast::PrimitiveType::Integer { width, .. } => Some(width),
                        _ => None,
                    });
                    json!({"name":field.name,"role":role_name(&field.role),"type":format_type(&field.ty),"offset":field.offset,"width":width,"mask":width.map(mask),"range":span_range(source,index,field.span)})
                }).collect::<Vec<_>>(),
            }))
        })
        .collect()
}

fn declarations(
    source: &str,
    index: &LineIndex,
    program: &Program,
    target: &TargetSpec,
) -> Vec<Value> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| {
            let (name, span, metadata): (&str, SourceSpan, &[MetaAttribute]) = match declaration {
                TopLevelDecl::Verb(verb) => (&verb.name, verb.span, &verb.metadata),
                TopLevelDecl::ExternalVerb(verb) => (&verb.name, verb.span, &verb.metadata),
                TopLevelDecl::Struct(value) => (&value.name, value.span, &[]),
                TopLevelDecl::Enum(value) => (&value.name, value.span, &[]),
                TopLevelDecl::Pack(value) => (&value.name, value.span, &[]),
                TopLevelDecl::Role(value) => (&value.name, value.span, &[]),
                TopLevelDecl::Import(_)
                | TopLevelDecl::Perform(_)
                | TopLevelDecl::OpenSibling(_) => return None,
            };
            let active = metadata.iter().all(|attribute| match attribute {
                MetaAttribute::Target(selector) => target.matches_platform(selector),
                MetaAttribute::Test => true,
                MetaAttribute::Limitless(_) => true,
            });
            Some(json!({"name":name,"active":active,"range":span_range(source,index,span)}))
        })
        .collect()
}

fn target_capabilities(target: &TargetSpec) -> Value {
    let hosted = matches!(target.entry_contract(), EntryContract::Hosted);
    let platform = if target.matches_platform("windows") { "windows" } else { "unix" };
    let capabilities = if hosted {
        vec![
            "semantic-analysis",
            "ownership-records",
            "pack-layouts",
            "checked-indexing",
            "hosted-runtime",
        ]
    } else {
        vec![
            "semantic-analysis",
            "ownership-records",
            "pack-layouts",
            "checked-indexing",
            "freestanding-entry",
        ]
    };
    json!({
        "triple": target.triple().to_string(),
        "architecture": format!("{:?}", target.architecture),
        "pointerWidth": format!("{:?}", target.pointer_width),
        "endianness": format!("{:?}", target.endianness),
        "objectFormat": format!("{:?}", target.object_format),
        "entryContract": if hosted { "hosted" } else { "freestanding" },
        "platform": platform,
        "capabilities": capabilities,
    })
}

fn source_range(source: &str, index: &LineIndex, span: SourceSpan) -> LspRange {
    let start = index.position(source, span.start.min(source.len()));
    let end = index.position(source, span.end.min(source.len()).max(span.start.min(source.len())));
    LspRange { start, end }
}

fn span_range(source: &str, index: &LineIndex, span: SourceSpan) -> Value {
    let range = source_range(source, index, span);
    json!({"startByte":span.start,"endByte":span.end,"range":range})
}

fn format_type(type_name: &TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(format_type).collect::<Vec<_>>().join(", ")
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

fn ownership_name(state: &OwnershipState) -> Value {
    match state {
        OwnershipState::Active => json!("Active"),
        OwnershipState::Moved => json!("Moved"),
        OwnershipState::Dropped => json!("Dropped"),
        OwnershipState::PartiallyMoved { fields } => {
            json!({"state":"PartiallyMoved","fields":fields})
        }
    }
}

fn access_name(state: &AccessState) -> Value {
    match state {
        AccessState::Mutable => json!("Mutable"),
        AccessState::Frozen { borrow_ids } => json!({"state":"Frozen","borrowIds":borrow_ids}),
        AccessState::Suspended { loan_id } => json!({"state":"Suspended","loanId":loan_id}),
    }
}

fn mask(width: u8) -> String {
    let mask = if width == 128 { u128::MAX } else { (1u128 << width) - 1 };
    format!("0x{mask:X}")
}
