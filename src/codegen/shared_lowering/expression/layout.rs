use crate::ast::{IntrinsicKind, lookup_call_intrinsic};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendContractError, BackendInstruction, BackendLayout, BackendValueId, BackendValueType,
};
use super::super::errors::unsupported;
use super::SharedExpressionProvider;

pub(super) fn lower_intrinsic(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    generic_arguments: &[crate::ast::TypeName],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<Option<BackendValueId>, BackendContractError> {
    let Some(intrinsic) = lookup_call_intrinsic(callee) else { return Ok(None) };
    if !matches!(intrinsic, IntrinsicKind::SizeOf | IntrinsicKind::AlignOf) {
        return Ok(None);
    }
    let value = layout_intrinsic_value(generic_arguments, intrinsic, provider.layouts)
        .ok_or_else(|| unsupported(span, "layout intrinsic type has no backend layout"))?;
    let result = provider.allocate();
    let ty = BackendValueType::Integer { signed: false, width: 64 };
    instructions.push(BackendInstruction::Constant { result, ty, value: u64::from(value), span });
    provider.remember_value(result, ty);
    Ok(Some(result))
}

fn layout_intrinsic_value(
    generic_arguments: &[crate::ast::TypeName],
    intrinsic: IntrinsicKind,
    layouts: &dyn super::super::BackendLayoutProvider,
) -> Option<u32> {
    let type_name = generic_arguments.first()?;
    layout_for_type(type_name, layouts).map(|layout| match intrinsic {
        IntrinsicKind::SizeOf => layout.size,
        IntrinsicKind::AlignOf => layout.alignment,
        _ => unreachable!(),
    })
}

fn layout_for_type(
    type_name: &crate::ast::TypeName,
    layouts: &dyn super::super::BackendLayoutProvider,
) -> Option<BackendLayout> {
    if let Some(layout) = layouts.layout_for(type_name) {
        return Some(layout);
    }
    let layout = match type_name.name.as_str() {
        "Bool" | "u8" | "i8" => BackendLayout { size: 1, alignment: 1 },
        "u16" | "i16" => BackendLayout { size: 2, alignment: 2 },
        "Int" | "u32" | "i32" => BackendLayout { size: 4, alignment: 4 },
        "Usize" | "u64" | "i64" => BackendLayout { size: 8, alignment: 8 },
        "Array" if type_name.arguments.len() == 2 => {
            let element = layout_for_type(&type_name.arguments[0], layouts)?;
            let count = type_name.arguments[1].name.parse::<u32>().ok()?;
            BackendLayout { size: element.size.checked_mul(count)?, alignment: element.alignment }
        }
        _ => return None,
    };
    Some(layout)
}
