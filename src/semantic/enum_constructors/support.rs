use crate::ast::{Argument, EnumDef, EnumVariant, Expr, TypeName};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::super::type_substitution::TypeSubstitution;

pub(super) fn strip_reference_role(type_name: &str) -> String {
    type_name
        .strip_prefix("abs ")
        .or_else(|| type_name.strip_prefix("ins "))
        .or_else(|| type_name.strip_prefix("erg "))
        .or_else(|| type_name.strip_prefix("dat "))
        .unwrap_or(type_name)
        .to_owned()
}

pub(super) fn named_argument_error(
    enum_name: &str,
    variant: &str,
    name: &str,
    argument: &Argument,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::EnumVariantArgumentName {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
            name: name.to_owned(),
        },
        span: argument_span(argument),
    }
}

pub(super) fn find_variant<'a>(definition: &'a EnumDef, name: &str) -> Option<&'a EnumVariant> {
    definition.variants.iter().find(|variant| variant.name == name)
}

pub(super) fn unknown_variant(enum_name: &str, variant: &str, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::UnknownEnumVariant {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
        },
        span,
    }
}

pub(super) fn argument_count(
    enum_name: &str,
    variant: &str,
    expected: usize,
    found: usize,
    span: SourceSpan,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::EnumVariantArgumentCount {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
            expected,
            found,
        },
        span,
    }
}

pub(super) fn argument_span(argument: &Argument) -> SourceSpan {
    match &argument.expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Case { span, .. } => *span,
    }
}

pub(super) fn enum_substitution(
    receiver: &Expr,
    definition: &EnumDef,
) -> Result<Option<TypeSubstitution>, SemanticError> {
    if definition.generic_parameters.is_empty() {
        return Ok(None);
    }
    let type_name = receiver_type_name(receiver).ok_or_else(|| SemanticError {
        kind: SemanticErrorKind::UnknownType { name: definition.name.clone() },
        span: definition.span,
    })?;
    TypeSubstitution::for_type(
        &definition.name,
        &definition.generic_parameters,
        &type_name.arguments,
        type_name.span,
    )
    .map(Some)
}

pub(super) fn receiver_type_name(receiver: &Expr) -> Option<TypeName> {
    let Expr::Identifier { name, span } = receiver else { return None };
    parse_type_name_key(name, *span)
}

fn parse_type_name_key(key: &str, span: SourceSpan) -> Option<TypeName> {
    let (reference_role, key) = reference_role_prefix(key);
    let Some(open) = key.find('[') else {
        return Some(TypeName {
            name: key.to_owned(),
            arguments: Vec::new(),
            reference_role,
            span,
        });
    };
    if !key.ends_with(']') {
        return None;
    }
    let arguments = split_type_arguments(&key[open + 1..key.len() - 1])
        .into_iter()
        .map(|argument| parse_type_name_key(argument, span))
        .collect::<Option<Vec<_>>>()?;
    Some(TypeName { name: key[..open].to_owned(), arguments, reference_role, span })
}

fn reference_role_prefix(key: &str) -> (Option<crate::ast::Role>, &str) {
    for (prefix, role) in [("abs ", crate::ast::Role::Abs), ("ins ", crate::ast::Role::Ins)] {
        if let Some(name) = key.strip_prefix(prefix) {
            return (Some(role), name);
        }
    }
    (None, key)
}

fn split_type_arguments(contents: &str) -> Vec<&str> {
    let mut depth = 0;
    let mut start = 0;
    let mut arguments = Vec::new();
    for (index, character) in contents.char_indices() {
        match character {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                arguments.push(contents[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < contents.len() {
        arguments.push(contents[start..].trim());
    }
    arguments
}

pub(super) fn canonical_type_name(type_name: &TypeName) -> String {
    let role = type_name
        .reference_role
        .as_ref()
        .map(|role| match role {
            crate::ast::Role::Abs => "abs ",
            crate::ast::Role::Ins => "ins ",
            crate::ast::Role::Erg => "erg ",
            crate::ast::Role::Dat => "dat ",
        })
        .unwrap_or("");
    if type_name.arguments.is_empty() {
        return format!("{role}{}", type_name.name);
    }
    format!(
        "{role}{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

pub(crate) fn generic_receiver_type(receiver: &Expr) -> Option<TypeName> {
    receiver_type_name(receiver)
}
