use crate::ast::{EnumDef, Expr, TypeName, TypeNameParseMode, parse_type_name_with_mode};
use crate::semantic::TypeSubstitution;
use crate::semantic::errors::{SemanticError, SemanticErrorKind};

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
    match receiver {
        Expr::Identifier { name, span } => {
            parse_type_name_with_mode(name, *span, TypeNameParseMode::Permissive)
        }
        Expr::Index { target, index, span } => {
            let Expr::Identifier { name, .. } = target.as_ref() else { return None };
            let Expr::Identifier { name: argument, .. } = index.as_ref() else { return None };
            Some(TypeName {
                name: name.clone(),
                arguments: vec![parse_type_name_with_mode(
                    argument,
                    *span,
                    TypeNameParseMode::Permissive,
                )?],
                reference_role: None,
                span: *span,
            })
        }
        _ => None,
    }
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
