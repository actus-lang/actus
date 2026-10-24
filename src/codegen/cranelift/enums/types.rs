use crate::ast::{Expr, TypeName};

use super::super::types::NativeType;
use crate::codegen::cranelift::layout::LayoutRegistry;

pub(crate) fn enum_receiver_name(receiver: &Expr) -> Option<String> {
    let span = match receiver {
        Expr::Identifier { span, .. } => *span,
        Expr::Index { span, .. } => *span,
        _ => return None,
    };
    let type_name = match receiver {
        Expr::Identifier { name, .. } => crate::semantic::parse_type_name_key(name, span)?,
        Expr::Index { target, index, .. } => {
            let Expr::Identifier { name, .. } = target.as_ref() else { return None };
            let Expr::Identifier { name: argument, .. } = index.as_ref() else { return None };
            TypeName {
                name: name.clone(),
                arguments: vec![crate::semantic::parse_type_name_key(argument, span)?],
                reference_role: None,
                span,
            }
        }
        _ => return None,
    };
    Some(canonical_type_name(&type_name))
}

fn canonical_type_name(type_name: &TypeName) -> String {
    let name = type_name.name.rsplit_once("::").map_or(type_name.name.as_str(), |(_, name)| name);
    if type_name.arguments.is_empty() {
        return name.to_owned();
    }
    format!(
        "{name}[{}]",
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

pub(crate) fn enum_expression_type(
    expression: &Expr,
    layouts: &LayoutRegistry,
) -> Option<NativeType> {
    let (receiver, variant) = match expression {
        Expr::MethodCall { receiver, method, .. } => (receiver.as_ref(), method.as_str()),
        Expr::FieldAccess { object, field, .. } => (object.as_ref(), field.as_str()),
        _ => return None,
    };
    let enum_name = enum_receiver_name(receiver)?;
    let (id, _) = layouts.enum_constructor(&enum_name, variant)?;
    Some(NativeType::Enum(id))
}
