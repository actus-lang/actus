use crate::ast::{Expr, TypeName};

use super::super::types::NativeType;
use crate::codegen::cranelift::layout::LayoutRegistry;

pub(crate) fn enum_receiver_name(type_name: Option<&TypeName>) -> Option<String> {
    type_name.map(canonical_type_name)
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
    let (_receiver, variant, resolved_type) = match expression {
        Expr::MethodCall { receiver, method, resolved_type, .. } => {
            (receiver.as_ref(), method.as_str(), resolved_type.as_ref())
        }
        Expr::FieldAccess { object, field, resolved_type, .. } => {
            (object.as_ref(), field.as_str(), resolved_type.as_ref())
        }
        _ => return None,
    };
    let enum_name = enum_receiver_name(resolved_type)?;
    let (id, _) = layouts.enum_constructor(&enum_name, variant)?;
    Some(NativeType::Enum(id))
}
