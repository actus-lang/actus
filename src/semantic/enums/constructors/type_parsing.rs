use crate::ast::{Expr, TypeName};
pub(super) fn receiver_type_name(receiver: &Expr) -> Option<TypeName> {
    let Expr::MethodCall { resolved_type, .. } = receiver else { return None };
    resolved_type.clone()
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
