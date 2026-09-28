use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::super::calls::CallLoweringContext;
use super::super::calls::{lower_call, lower_method_call};
use super::super::enums::{enum_receiver_name, lower_enum_constructor};
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::model::NativeCleanupSchedule;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::structs::{lower_field_access, lower_pack_literal, lower_struct_literal};
use super::super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(in crate::codegen) fn lower_construct(
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
        Expr::Call { callee, arguments, .. } => {
            let context = CallLoweringContext::new(
                locals,
                local_types,
                functions,
                cleanup_schedule,
                string_data,
                layouts,
            );
            lower_call(function, callee, arguments, &context)
        }
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
        Expr::StructLit { name, type_arguments, fields, .. } if layouts.pack_id(name).is_some() => {
            lower_pack_literal(
                function,
                name,
                fields,
                locals,
                local_types,
                functions,
                cleanup_schedule,
                string_data,
                layouts,
            )
        }
        Expr::StructLit { name, type_arguments, fields, .. } => lower_struct_literal(
            function,
            name,
            type_arguments,
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
    if let Some(value) = lower_arena_method(
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
    ) {
        return value;
    }
    if enum_receiver_name(receiver)
        .and_then(|name| layouts.enum_constructor(name, method))
        .is_some()
    {
        let context = CallLoweringContext::new(
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        );
        lower_enum_constructor(function, receiver, method, arguments, &context)
    } else {
        let context = CallLoweringContext::new(
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        );
        lower_method_call(function, receiver, method, arguments, &context)
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_arena_method(
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
) -> Option<Result<cranelift_codegen::ir::Value, NativeEmitError>> {
    if method != "place"
        || !matches!(
            super::super::structs::expression_native_type(receiver, local_types, layouts),
            Some(NativeType::Arena(_))
        )
    {
        return None;
    }
    Some(super::super::arenas::lower_place(
        function,
        receiver,
        arguments,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    ))
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
        let context = CallLoweringContext::new(
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        );
        lower_enum_constructor(function, object, field, &[], &context)
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
