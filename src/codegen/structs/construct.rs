use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{StructFieldInit, TypeName};

use super::super::expressions::lower_expression;
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::model::NativeCleanupSchedule;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::memory::store_struct_field;

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_struct_literal(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    type_arguments: &[TypeName],
    fields: &[StructFieldInit],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let type_name = TypeName {
        name: name.to_owned(),
        arguments: type_arguments.to_vec(),
        reference_role: None,
        span: crate::lexer::SourceSpan::new(0, 0),
    };
    let id = layouts
        .type_for_type_name(&type_name)
        .and_then(|ty| match ty {
            NativeType::Struct(id) => Some(id),
            _ => None,
        })
        .ok_or_else(|| NativeEmitError(format!("missing layout for struct `{name}`")))?;
    let layout =
        layouts.get(id).ok_or_else(|| NativeEmitError(format!("missing layout `{id}`")))?;
    let slot = function.func.create_sized_stack_slot(layouts.stack_slot(layout));
    let address = function.ins().stack_addr(layouts.pointer_type, slot, 0);
    for field in fields {
        let field_layout = layout
            .fields
            .iter()
            .find(|candidate| candidate.name == field.name)
            .ok_or_else(|| NativeEmitError(format!("unknown native field `{}`", field.name)))?;
        let value = lower_expression(
            function,
            &field.value,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )?;
        store_struct_field(function, address, value, field_layout, layouts)?;
    }
    Ok(address)
}
