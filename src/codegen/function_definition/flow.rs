use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use super::super::layout::LayoutRegistry;
use super::super::native::NativeEmitError;
use super::super::structs::copy_bytes;
use super::super::types::NativeType;

pub(super) fn emit_flow(
    function: &mut FunctionBuilder<'_>,
    return_type: Option<NativeType>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    flow: super::super::lowering::Flow,
    layouts: &LayoutRegistry,
    force_void_entry_return: bool,
) -> Result<(), NativeEmitError> {
    match flow {
        super::super::lowering::Flow::Return(result) => {
            emit_value_return(function, return_type, return_slot, result, layouts)
        }
        super::super::lowering::Flow::Fallthrough => {
            emit_fallthrough(function, return_type, return_slot, force_void_entry_return)
        }
        super::super::lowering::Flow::VoidReturn => {
            emit_void_return(function, return_type, return_slot, force_void_entry_return)
        }
        super::super::lowering::Flow::Break | super::super::lowering::Flow::Continue => {
            Err(NativeEmitError("loop control escaped its loop during native lowering".to_owned()))
        }
    }
}

fn emit_value_return(
    function: &mut FunctionBuilder<'_>,
    return_type: Option<NativeType>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    result: cranelift_codegen::ir::Value,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    match (return_type, return_slot) {
        (Some(return_type), Some(destination)) if return_type.uses_sret() => {
            emit_sret_return(function, result, destination, return_type, layouts)
        }
        (Some(return_type), Some(destination)) if layouts.returns_borrowed_view(return_type) => {
            emit_borrowed_slot_return(function, destination, result, return_type, layouts)
        }
        (Some(NativeType::Void), _) => {
            Err(NativeEmitError("Void native function returned a value".to_owned()))
        }
        (Some(_), None) => {
            function.ins().return_(&[result]);
            Ok(())
        }
        (None, _) => Err(NativeEmitError("void native function returned a value".to_owned())),
        _ => Err(NativeEmitError("invalid native return ABI state".to_owned())),
    }
}

fn emit_sret_return(
    function: &mut FunctionBuilder<'_>,
    result: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    return_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let size = layouts
        .type_size(return_type)
        .ok_or_else(|| NativeEmitError("missing sret return layout".to_owned()))?;
    copy_bytes(function, result, destination, size);
    function.ins().return_(&[]);
    Ok(())
}

fn emit_borrowed_slot_return(
    function: &mut FunctionBuilder<'_>,
    destination: cranelift_codegen::ir::Value,
    result: cranelift_codegen::ir::Value,
    return_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let size = layouts
        .return_slot_size(return_type)
        .ok_or_else(|| NativeEmitError("missing borrowed view return layout".to_owned()))?;
    emit_borrowed_view_return(function, destination, result, size);
    Ok(())
}

fn emit_fallthrough(
    function: &mut FunctionBuilder<'_>,
    return_type: Option<NativeType>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    force_void_entry_return: bool,
) -> Result<(), NativeEmitError> {
    if force_void_entry_return {
        let result = function.ins().iconst(cranelift_codegen::ir::types::I32, 0);
        function.ins().return_(&[result]);
        return Ok(());
    }
    match (return_type, return_slot) {
        (None, None) | (Some(NativeType::Void), None) => {
            function.ins().return_(&[]);
            Ok(())
        }
        (Some(_), None) => {
            Err(NativeEmitError("native function requires a return value".to_owned()))
        }
        _ => Err(NativeEmitError("invalid native return ABI state".to_owned())),
    }
}

fn emit_void_return(
    function: &mut FunctionBuilder<'_>,
    return_type: Option<NativeType>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    force_void_entry_return: bool,
) -> Result<(), NativeEmitError> {
    if force_void_entry_return {
        let result = function.ins().iconst(cranelift_codegen::ir::types::I32, 0);
        function.ins().return_(&[result]);
        return Ok(());
    }
    match (return_type, return_slot) {
        (None, None) | (Some(NativeType::Void), None) => {
            function.ins().return_(&[]);
            Ok(())
        }
        _ => Err(NativeEmitError("invalid native return ABI state".to_owned())),
    }
}

fn emit_borrowed_view_return(
    function: &mut FunctionBuilder<'_>,
    destination: cranelift_codegen::ir::Value,
    result: cranelift_codegen::ir::Value,
    size: u32,
) {
    let some = function.create_block();
    let none = function.create_block();
    let merge = function.create_block();
    let pointer_type = function.func.dfg.value_type(destination);
    function.append_block_param(merge, pointer_type);
    function.ins().brif(result, some, &[], none, &[]);
    function.switch_to_block(some);
    super::super::structs::copy_bytes(function, result, destination, size);
    let some_argument = cranelift_codegen::ir::BlockArg::Value(destination);
    function.ins().jump(merge, [&some_argument]);
    function.seal_block(some);
    function.switch_to_block(none);
    let none_argument = cranelift_codegen::ir::BlockArg::Value(result);
    function.ins().jump(merge, [&none_argument]);
    function.seal_block(none);
    function.switch_to_block(merge);
    let returned = function.block_params(merge)[0];
    function.ins().return_(&[returned]);
    function.seal_block(merge);
}
