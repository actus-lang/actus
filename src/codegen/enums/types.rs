use crate::ast::Expr;

use super::super::layout::LayoutRegistry;
use super::super::types::NativeType;

pub(crate) fn enum_receiver_name(receiver: &Expr) -> Option<&str> {
    let Expr::Identifier { name, .. } = receiver else { return None };
    Some(name)
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
    let (id, _) = layouts.enum_constructor(enum_name, variant)?;
    Some(NativeType::Enum(id))
}
