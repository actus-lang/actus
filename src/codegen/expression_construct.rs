use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::calls::{lower_call, lower_method_call};
use super::enums::{enum_receiver_name, lower_enum_constructor};
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::structs::{lower_field_access, lower_struct_literal};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_construct(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Call { callee, arguments, .. } => lower_call(
            function,
            callee,
            arguments,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::MethodCall { receiver, method, arguments, .. } => lower_method_construct(
            function,
            receiver,
            method,
            arguments,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::StructLit { .. } | Expr::FieldAccess { .. } => lower_data_construct(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native construct".to_owned())),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_data_construct(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::StructLit { name, fields, .. } => lower_struct_literal(
            function,
            name,
            fields,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::FieldAccess { object, field, .. } => lower_field_construct(
            function,
            object,
            field,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported data construct".to_owned())),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_method_construct(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    method: &str,
    arguments: &[Argument],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if enum_receiver_name(receiver)
        .and_then(|name| layouts.enum_constructor(name, method))
        .is_some()
    {
        lower_enum_constructor(
            function,
            receiver,
            method,
            arguments,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
    } else {
        lower_method_call(
            function,
            receiver,
            method,
            arguments,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_field_construct(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if enum_receiver_name(object).and_then(|name| layouts.enum_constructor(name, field)).is_some() {
        lower_enum_constructor(
            function,
            object,
            field,
            &[],
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
    } else {
        lower_field_access(
            function,
            object,
            field,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
    }
}
