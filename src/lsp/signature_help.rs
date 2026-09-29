use std::io::{self, Write};

use serde_json::{Value, json};

use crate::ast::{GenericParam, Param, Role, TopLevelDecl, TypeName};
use crate::lexer::{Token, TokenKind, scan};
use crate::parser::parse;

use super::cancellation::CancellationToken;
use super::documents::DocumentStore;
use super::position::{LineIndex, LspPosition};
use super::protocol::ResponseMetadata;
use crate::target::TargetSpec;

pub(super) fn dispatch<W: Write>(
    params: Value,
    store: &DocumentStore,
    target: &TargetSpec,
    output: &mut W,
    id: Option<Value>,
    metadata: ResponseMetadata,
    cancellation: Option<&CancellationToken>,
) -> io::Result<()> {
    if stale_document(&params, store) {
        let metadata = ResponseMetadata { result_state: "stale", ..metadata };
        return super::server::respond(output, id, Value::Null, metadata, cancellation);
    }
    let result = signature_help(&params, store, target);
    super::server::respond(output, id, result, metadata, cancellation)
}

fn stale_document(params: &Value, store: &DocumentStore) -> bool {
    let Some(document) = params.get("textDocument") else { return false };
    let Some(uri) = document.get("uri").and_then(Value::as_str) else { return false };
    let Some(version) = document.get("version").and_then(Value::as_i64) else { return false };
    store.get(uri).is_some_and(|current| current.version != version)
}

fn signature_help(params: &Value, store: &DocumentStore, target: &TargetSpec) -> Value {
    let Some(uri) =
        params.get("textDocument").and_then(|doc| doc.get("uri")).and_then(Value::as_str)
    else {
        return Value::Null;
    };
    let Some(document) = store.get(uri) else { return Value::Null };
    let Some(position) = params
        .get("position")
        .and_then(|value| serde_json::from_value::<LspPosition>(value.clone()).ok())
    else {
        return Value::Null;
    };
    let Some((callee, active_parameter)) = call_context(&document.text, &position) else {
        return Value::Null;
    };
    let Some(signature) = find_signature(&document.text, &callee, target) else {
        return Value::Null;
    };
    json!({
        "signatures": [signature],
        "activeSignature": 0,
        "activeParameter": active_parameter,
    })
}

fn call_context(source: &str, position: &LspPosition) -> Option<(String, u32)> {
    let tokens = scan(source).0;
    let offset = LineIndex::new(source).byte_offset(source, position)?;
    let open_index = tokens.iter().rposition(|token| {
        token.span.start <= offset && matches!(token.kind, TokenKind::LeftParen)
    })?;
    let callee = match tokens.get(open_index.checked_sub(1)?)?.kind.clone() {
        TokenKind::Identifier(name) => name,
        _ => return None,
    };
    if matches!(
        tokens.get(open_index.checked_sub(2)?).map(|token| &token.kind),
        Some(TokenKind::Verb | TokenKind::Extern)
    ) {
        return None;
    }
    let active = active_parameter(&tokens[open_index + 1..], offset);
    Some((callee, active))
}

fn active_parameter(tokens: &[Token], offset: usize) -> u32 {
    let mut depth = 0_u32;
    let mut commas = 0_u32;
    for token in tokens {
        if token.span.start > offset {
            break;
        }
        match token.kind {
            TokenKind::LeftParen | TokenKind::LeftBracket => depth += 1,
            TokenKind::RightParen | TokenKind::RightBracket => depth = depth.saturating_sub(1),
            TokenKind::Comma if depth == 0 => commas += 1,
            _ => {}
        }
    }
    commas
}

fn find_signature(source: &str, name: &str, target: &TargetSpec) -> Option<Value> {
    if let Some(signature) = intrinsic_signature(name) {
        return Some(signature);
    }
    let program = crate::semantic::filter_program_for_target(&parse(scan(source).0).ok()?, target);
    program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Verb(verb) if verb.name == name => Some(callable_signature(
            &verb.name,
            &verb.generic_parameters,
            &verb.params,
            verb.return_type.as_ref(),
            verb.doc.as_deref(),
        )),
        TopLevelDecl::ExternalVerb(verb) if verb.name == name => Some(callable_signature(
            &verb.name,
            &verb.generic_parameters,
            &verb.params,
            verb.return_type.as_ref(),
            verb.doc.as_deref(),
        )),
        _ => None,
    })
}

fn intrinsic_signature(name: &str) -> Option<Value> {
    let (label, documentation, parameters) = match name {
        "print" | "println" => (
            format!("{name}(abs text: Buffer) -> Result[Int, IoError]"),
            "Writes exactly the live Buffer length.",
            vec![("abs text".to_owned(), "Buffer".to_owned())],
        ),
        "append" => (
            "append(ins buffer: Buffer, abs byte: u8) -> Void".to_owned(),
            "Appends one byte in place.",
            vec![
                ("ins buffer".to_owned(), "Buffer".to_owned()),
                ("abs byte".to_owned(), "u8".to_owned()),
            ],
        ),
        _ => return None,
    };
    Some(signature_value(&label, documentation, &parameters))
}

fn callable_signature(
    name: &str,
    generic_parameters: &[GenericParam],
    params: &[Param],
    return_type: Option<&crate::ast::ReturnType>,
    documentation: Option<&str>,
) -> Value {
    let parameters = params.iter().map(parameter_label).collect::<Vec<_>>();
    let label = format!(
        "{name}{}({}){}",
        generic_label(generic_parameters),
        parameters.iter().map(|(label, _)| label.as_str()).collect::<Vec<_>>().join(", "),
        return_label(return_type)
    );
    signature_value(&label, documentation.unwrap_or(""), &parameters)
}

fn generic_label(parameters: &[GenericParam]) -> String {
    if parameters.is_empty() {
        return String::new();
    }
    let parameters = parameters.iter().map(generic_parameter_label).collect::<Vec<_>>();
    format!("[{}]", parameters.join(", "))
}

fn generic_parameter_label(parameter: &GenericParam) -> String {
    let bounds = parameter.bounds.iter().map(type_label).collect::<Vec<_>>();
    if bounds.is_empty() {
        parameter.name.clone()
    } else {
        format!("{}: {}", parameter.name, bounds.join(" + "))
    }
}

fn parameter_label(parameter: &Param) -> (String, String) {
    let role = match parameter.role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    };
    let ty = type_label(&parameter.ty);
    (format!("{role} {}: {ty}", parameter.name), format!("{}: {ty}", parameter.name))
}

fn signature_value(label: &str, documentation: &str, parameters: &[(String, String)]) -> Value {
    json!({
        "label": label,
        "documentation": documentation,
        "parameters": parameters.iter().map(|(label, _)| json!({"label": label})).collect::<Vec<_>>(),
    })
}

fn return_label(return_type: Option<&crate::ast::ReturnType>) -> String {
    return_type.map_or_else(String::new, |value| format!(" -> {}", type_label(&value.ty)))
}

fn type_label(ty: &TypeName) -> String {
    if ty.arguments.is_empty() {
        return ty.name.clone();
    }
    format!("{}[{}]", ty.name, ty.arguments.iter().map(type_label).collect::<Vec<_>>().join(", "))
}
