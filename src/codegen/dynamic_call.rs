use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlagsData, Signature, types};
use cranelift_frontend::FunctionBuilder;

use super::native::NativeEmitError;

pub(super) fn lower_dynamic_call(
    function: &mut FunctionBuilder<'_>,
    fat_pointer: cranelift_codegen::ir::Value,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let pointer_type = function.func.dfg.value_type(fat_pointer);
    let data_pointer = function.ins().load(pointer_type, MemFlagsData::new(), fat_pointer, 0);
    let vtable_pointer = function.ins().load(
        pointer_type,
        MemFlagsData::new(),
        fat_pointer,
        pointer_type.bytes() as i32,
    );
    let method_pointer = function.ins().load(pointer_type, MemFlagsData::new(), vtable_pointer, 0);
    let mut signature = Signature::new(function.func.signature.call_conv);
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));
    let signature = function.import_signature(signature);
    let call = function.ins().call_indirect(signature, method_pointer, &[data_pointer]);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError("dynamic call returned no value".to_owned()))
}
