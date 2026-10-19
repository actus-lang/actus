use cranelift_codegen::ir::{
    GlobalValue, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value,
};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::super::expressions::lower_expression;
use super::super::native::BackendEmitError;
use super::super::types::NativeType;
use super::context::CallLoweringContext;

struct CallArgumentModes<'a> {
    dynamic_params: &'a [bool],
    dynamic_roles: &'a [Option<String>],
    ins_params: &'a [bool],
}

pub(super) fn lower_call_arguments(
    function: &mut FunctionBuilder<'_>,
    arguments: &[Argument],
    parameter_names: &[String],
    parameter_types: &[NativeType],
    dynamic_params: &[bool],
    dynamic_roles: &[Option<String>],
    ins_params: &[bool],
    context: &CallLoweringContext<'_, '_>,
) -> Result<Vec<Value>, BackendEmitError> {
    let ordered = order_arguments(arguments, parameter_names)?;
    let modes = CallArgumentModes { dynamic_params, dynamic_roles, ins_params };
    let mut values = Vec::new();
    for (index, argument) in ordered.iter().enumerate() {
        lower_one_call_argument(
            function,
            argument,
            index,
            parameter_types.get(index).copied(),
            &modes,
            context,
            &mut values,
        )?;
    }
    Ok(values)
}

fn lower_one_call_argument(
    function: &mut FunctionBuilder<'_>,
    argument: &Expr,
    index: usize,
    expected_type: Option<NativeType>,
    modes: &CallArgumentModes<'_>,
    context: &CallLoweringContext<'_, '_>,
    values: &mut Vec<Value>,
) -> Result<(), BackendEmitError> {
    let value = if modes.ins_params.get(index).copied().unwrap_or(false) {
        lower_indirect_ins_argument(function, argument, context)?
    } else {
        lower_expression(
            function,
            argument,
            context.locals,
            context.local_types,
            context.functions,
            context.cleanup_schedule,
            context.string_data,
            context.layouts,
        )?
    };
    values.push(coerce_call_argument(function, value, expected_type, context)?);
    append_dynamic_argument(
        function,
        argument,
        index,
        modes.dynamic_params,
        modes.dynamic_roles,
        context,
        values,
    )
}

fn coerce_call_argument(
    function: &mut FunctionBuilder<'_>,
    value: Value,
    expected_type: Option<NativeType>,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, BackendEmitError> {
    let Some(expected_type) = expected_type else { return Ok(value) };
    let actual_ir_type = function.func.dfg.value_type(value);
    let expected_ir_type = expected_type.ir_type(context.layouts.pointer_type)?;
    if actual_ir_type == expected_ir_type {
        return Ok(value);
    }
    if !matches!(expected_type, NativeType::Int | NativeType::Integer { .. }) {
        return Err(BackendEmitError(format!(
            "call argument type mismatch: expected {expected_ir_type}, got {actual_ir_type}"
        )));
    }
    if actual_ir_type.bytes() < expected_ir_type.bytes() {
        return Ok(match expected_type {
            NativeType::Integer { signed: true, .. } | NativeType::Int => {
                function.ins().sextend(expected_ir_type, value)
            }
            NativeType::Integer { signed: false, .. } => {
                function.ins().uextend(expected_ir_type, value)
            }
            _ => unreachable!(),
        });
    }
    Ok(function.ins().ireduce(expected_ir_type, value))
}

fn lower_indirect_ins_argument(
    function: &mut FunctionBuilder<'_>,
    argument: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, BackendEmitError> {
    match argument {
        Expr::Index { target, index, .. } => {
            let (address, _) = super::super::arrays::lower_array_address(
                function,
                target,
                index,
                context.locals,
                context.local_types,
                context.functions,
                context.cleanup_schedule,
                context.string_data,
                context.layouts,
            )?;
            Ok(address)
        }
        Expr::FieldAccess { object, field, .. } => {
            crate::codegen::cranelift::structs::lower_field_address_for_ins(
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
        Expr::Identifier { name, .. } => lower_scalar_ins_identifier(function, name, context),
        _ => Err(BackendEmitError(
            "native `ins` arguments require an addressable scalar or indexed slot".to_owned(),
        )),
    }
}

fn lower_scalar_ins_identifier(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, BackendEmitError> {
    let (binding, value) = context
        .locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(binding, value)| (*binding, *value))
        .ok_or_else(|| BackendEmitError(format!("native binding `{name}` is unavailable")))?;
    let native_type = context
        .local_types
        .get(binding)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("native binding `{name}` has no type")))?;
    let ir_type = native_type.ir_type(context.layouts.pointer_type)?;
    if ir_type == context.layouts.pointer_type {
        return Ok(value);
    }
    let size = ir_type.bytes();
    let slot = function.func.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        size,
        ir_type.bytes().trailing_zeros() as u8,
    ));
    let address = function.ins().stack_addr(context.layouts.pointer_type, slot, 0);
    function.ins().store(MemFlagsData::new(), value, address, 0);
    Ok(address)
}

fn append_dynamic_argument(
    function: &mut FunctionBuilder<'_>,
    argument: &Expr,
    index: usize,
    dynamic_params: &[bool],
    dynamic_roles: &[Option<String>],
    context: &CallLoweringContext<'_, '_>,
    values: &mut Vec<Value>,
) -> Result<(), BackendEmitError> {
    if !is_dynamic_parameter(dynamic_params, index)? {
        return Ok(());
    }
    let role = dynamic_roles
        .get(index)
        .and_then(Option::as_deref)
        .ok_or_else(|| BackendEmitError("dynamic parameter has no role".to_owned()))?;
    let global = dynamic_vtable_value(argument, role, context)?;
    values.push(function.ins().symbol_value(context.layouts.pointer_type, global));
    Ok(())
}

fn dynamic_vtable_value(
    argument: &Expr,
    role: &str,
    context: &CallLoweringContext<'_, '_>,
) -> Result<GlobalValue, BackendEmitError> {
    let native_type = super::super::expressions::initializer_type(
        argument,
        context.local_types,
        context.functions,
        context.layouts,
    )?;
    let symbol = context.string_data.vtable_symbol(role, native_type);
    context
        .string_data
        .get(&symbol)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("vtable `{symbol}` has no native data")))
}

fn is_dynamic_parameter(dynamic_params: &[bool], index: usize) -> Result<bool, BackendEmitError> {
    dynamic_params
        .get(index)
        .copied()
        .ok_or_else(|| BackendEmitError("call metadata is missing parameter mode".to_owned()))
}

fn order_arguments<'source>(
    arguments: &'source [Argument],
    parameter_names: &[String],
) -> Result<Vec<&'source Expr>, BackendEmitError> {
    if arguments.iter().all(|argument| argument.name.is_none()) {
        return Ok(arguments.iter().map(|argument| &argument.expression).collect());
    }
    if arguments.iter().any(|argument| argument.name.is_none()) {
        return Err(BackendEmitError(
            "native calls cannot mix named and positional arguments".to_owned(),
        ));
    }
    parameter_names
        .iter()
        .map(|parameter| {
            arguments
                .iter()
                .find(|argument| argument.name.as_deref() == Some(parameter))
                .map(|argument| &argument.expression)
                .ok_or_else(|| BackendEmitError(format!("missing native argument `{parameter}")))
        })
        .collect()
}
