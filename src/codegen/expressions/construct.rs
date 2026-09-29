use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::super::arrays::lower_array_constructor;
use super::super::calls::CallLoweringContext;
use super::super::calls::{lower_call, lower_method_call};
use super::super::enums::{enum_receiver_name, lower_enum_constructor};
use super::super::native::NativeEmitError;
use super::super::structs::{lower_field_access, lower_pack_literal, lower_struct_literal};
use super::super::types::NativeType;

pub(in crate::codegen) fn lower_construct(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Call { callee, arguments, .. } => {
            lower_call_or_array_constructor(function, callee, arguments, context)
        }
        Expr::MethodCall { receiver, method, arguments, .. } => {
            lower_method_construct(function, receiver, method, arguments, context)
        }
        Expr::StructLit { .. } | Expr::FieldAccess { .. } => {
            lower_data_construct(function, expression, context)
        }
        _ => Err(NativeEmitError("unsupported native construct".to_owned())),
    }
}

fn lower_call_or_array_constructor(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    arguments: &[Argument],
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if arguments.is_empty()
        && let Some(id) = context.layouts.array_id(callee)
    {
        return lower_array_constructor(function, id, context.layouts);
    }
    lower_call(function, callee, arguments, context)
}

fn lower_data_construct(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::StructLit { name, fields, .. } if context.layouts.pack_id(name).is_some() => {
            lower_pack_data(function, name, fields, context)
        }
        Expr::StructLit { name, type_arguments, fields, .. } => {
            lower_struct_data(function, name, type_arguments, fields, context)
        }
        Expr::FieldAccess { object, field, .. } => {
            lower_field_construct(function, object, field, context)
        }
        _ => Err(NativeEmitError("unsupported data construct".to_owned())),
    }
}

fn lower_pack_data(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    fields: &[crate::ast::StructFieldInit],
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    lower_pack_literal(
        function,
        name,
        fields,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}

fn lower_struct_data(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    type_arguments: &[crate::ast::TypeName],
    fields: &[crate::ast::StructFieldInit],
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    lower_struct_literal(
        function,
        name,
        type_arguments,
        fields,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}

fn lower_method_construct(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    method: &str,
    arguments: &[Argument],
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if let Some(value) = lower_arena_method(function, receiver, method, arguments, context) {
        return value;
    }
    if enum_receiver_name(receiver)
        .and_then(|name| context.layouts.enum_constructor(name, method))
        .is_some()
    {
        lower_enum_constructor(function, receiver, method, arguments, context)
    } else {
        lower_method_call(function, receiver, method, arguments, context)
    }
}

fn lower_arena_method(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    method: &str,
    arguments: &[Argument],
    context: &CallLoweringContext<'_, '_>,
) -> Option<Result<cranelift_codegen::ir::Value, NativeEmitError>> {
    if !is_arena_method(receiver, method, context) {
        return None;
    }
    Some(super::super::arenas::lower_place(
        function,
        receiver,
        arguments,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    ))
}

fn is_arena_method(receiver: &Expr, method: &str, context: &CallLoweringContext<'_, '_>) -> bool {
    method == "place"
        && matches!(
            super::super::structs::expression_native_type(
                receiver,
                context.local_types,
                context.layouts,
            ),
            Some(NativeType::Arena(_))
        )
}

fn lower_field_construct(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if enum_receiver_name(object)
        .and_then(|name| context.layouts.enum_constructor(name, field))
        .is_some()
    {
        lower_enum_constructor(function, object, field, &[], context)
    } else {
        lower_field_access(
            function,
            object,
            field,
            context.locals,
            context.local_types,
            context.functions,
            context.cleanup_schedule,
            context.string_data,
            context.layouts,
        )
    }
}
