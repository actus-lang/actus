//! Failed runtime checks in Cranelift code (ADR-0117).
//!
//! Without a message a failed check executes a trap, as freestanding code
//! needs. With one it calls the runtime's check-failure report, which prints
//! the message and ends the process.

use cranelift_codegen::ir::{AbiParam, InstBuilder, TrapCode, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{DataDescription, Linkage, Module};
use cranelift_object::ObjectModule;

use super::native::BackendEmitError;

/// Ends the current block at a failed check: reports `failure` when it
/// holds a message, otherwise traps with `code`.
pub(super) fn emit_check_failure(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    code: TrapCode,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let Some(message) = failure else {
        builder.ins().trap(code);
        return Ok(());
    };
    let pointer_type = module.isa().pointer_type();
    let data = module
        .declare_anonymous_data(false, false)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let mut description = DataDescription::new();
    description.define(format!("{message}\0").into_bytes().into_boxed_slice());
    module.define_data(data, &description).map_err(|error| BackendEmitError(error.to_string()))?;
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    let report = module
        .declare_function(crate::runtime::RUNTIME_CHECK_FAILED_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let report = module.declare_func_in_func(report, builder.func);
    let message = module.declare_data_in_func(data, builder.func);
    let message = builder.ins().symbol_value(pointer_type, message);
    builder.ins().call(report, &[message]);
    // The report never returns; the trap ends the block for the verifier.
    builder.ins().trap(code);
    Ok(())
}

/// Fails the check when `failed` is nonzero and continues otherwise.
pub(super) fn check_failed_if(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    failed: Value,
    code: TrapCode,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    if failure.is_none() {
        builder.ins().trapnz(failed, code);
        return Ok(());
    }
    let failure_block = builder.create_block();
    let ok_block = builder.create_block();
    builder.set_cold_block(failure_block);
    builder.ins().brif(failed, failure_block, &[], ok_block, &[]);
    builder.switch_to_block(failure_block);
    emit_check_failure(module, builder, code, failure)?;
    builder.seal_block(failure_block);
    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);
    Ok(())
}
