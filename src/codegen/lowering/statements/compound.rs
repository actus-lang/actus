use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, CompoundAssignmentOp, Expr, Place};

use super::super::super::arrays::lower_array_compound_assignment;
use super::super::super::buffer_index::lower_buffer_compound_assignment;
use super::super::super::expressions::{lower_compound_integer_operation, lower_expression};
use super::super::super::layout::LayoutRegistry;
use super::super::super::literals::StringDataValues;
use super::super::super::native::{FunctionRef, NativeEmitError};
use super::super::super::structs::{expression_native_type, lower_field_compound_assignment};
use super::super::super::types::NativeType;
use super::{Flow, NativeCleanupSchedule};

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_compound_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    target: &Place,
    operator: CompoundAssignmentOp,
    expression: &Expr,
    locals: &mut HashMap<&'source String, Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let operator = compound_binary_operator(operator);
    match target {
        Place::Binding { name, .. } => lower_identifier(
            function,
            name,
            operator,
            expression,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Place::Field { object, field, .. } => lower_field_compound_assignment(
            function,
            &object.to_expr(),
            field,
            operator,
            expression,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
        .map(|()| Flow::Fallthrough),
        Place::Index { target, index, .. } => lower_index(
            function,
            &target.to_expr(),
            index,
            operator,
            expression,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
        .map(|()| Flow::Fallthrough),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_identifier<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    operator: BinaryOp,
    expression: &Expr,
    locals: &mut HashMap<&'source String, Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let binding = locals
        .keys()
        .find(|candidate| candidate.as_str() == name)
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    let destination = locals[&binding];
    let left_type = types
        .get(&binding)
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native type for `{name}` is unavailable")))?;
    let left = load_indirect_scalar(function, destination, left_type, layouts)?;
    let right = lower_expression(
        function,
        expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let right = left_type
        .ir_type(layouts.pointer_type)
        .map(|ty| super::super::super::expressions::coerce_to_ir_type(function, right, ty))?;
    let updated = lower_compound_integer_operation(function, left, operator, right, left_type)?;
    if is_indirect_scalar(function, destination, left_type, layouts) {
        function.ins().store(MemFlagsData::new(), updated, destination, 0);
    } else {
        locals.insert(binding, updated);
    }
    Ok(Flow::Fallthrough)
}

fn load_indirect_scalar(
    function: &mut FunctionBuilder<'_>,
    destination: Value,
    native_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    if is_indirect_scalar(function, destination, native_type, layouts) {
        let ty = native_type.ir_type(layouts.pointer_type)?;
        return Ok(function.ins().load(ty, MemFlagsData::new(), destination, 0));
    }
    Ok(destination)
}

fn is_indirect_scalar(
    function: &FunctionBuilder<'_>,
    destination: Value,
    native_type: NativeType,
    layouts: &LayoutRegistry,
) -> bool {
    native_type.ir_type(layouts.pointer_type).is_ok_and(|ty| {
        ty != layouts.pointer_type
            && function.func.dfg.value_type(destination) == layouts.pointer_type
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_index<'source>(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    index: &Expr,
    operator: BinaryOp,
    value: &Expr,
    locals: &HashMap<&'source String, Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    if expression_native_type(target, types, layouts) == Some(NativeType::Buffer) {
        lower_buffer_compound_assignment(
            function,
            target,
            index,
            operator,
            value,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
    } else {
        lower_array_compound_assignment(
            function,
            target,
            index,
            operator,
            value,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )
    }
}

fn compound_binary_operator(operator: CompoundAssignmentOp) -> BinaryOp {
    match operator {
        CompoundAssignmentOp::Add => BinaryOp::Add,
        CompoundAssignmentOp::Subtract => BinaryOp::Subtract,
        CompoundAssignmentOp::Multiply => BinaryOp::Multiply,
        CompoundAssignmentOp::Divide => BinaryOp::Divide,
        CompoundAssignmentOp::Remainder => BinaryOp::Remainder,
        CompoundAssignmentOp::BitwiseAnd => BinaryOp::BitwiseAnd,
        CompoundAssignmentOp::BitwiseOr => BinaryOp::BitwiseOr,
        CompoundAssignmentOp::BitwiseXor => BinaryOp::BitwiseXor,
        CompoundAssignmentOp::ShiftLeft => BinaryOp::ShiftLeft,
        CompoundAssignmentOp::ShiftRight => BinaryOp::ShiftRight,
    }
}
