mod aggregate;
mod arena;
mod arrays;
mod cleanup;
mod control_flow;
mod declarations;
mod dispatch;
mod enums;
mod functions;
mod packed;
mod returns;
mod scalar;

use super::integer_ops::{
    lower_integer_binary, lower_integer_cast, lower_integer_compare, lower_integer_unary,
};
use super::native::BackendEmitError;
use super::object::create_module;
use crate::codegen::{BackendInstruction, BackendModule};
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

pub(super) use cleanup::LayoutCleanupProvider;
pub(crate) use declarations::{parameter_types, scalar_type};

pub fn emit_contract_module_object(
    backend_module: &BackendModule,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    backend_module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    reject_float_ir(backend_module, configuration)?;
    let mut module = create_module(configuration, target)?;
    let declarations = declarations::declare_functions(&mut module, backend_module)?;
    for function in &backend_module.functions {
        functions::define_function(&mut module, function, &declarations)?;
    }
    module.finish().emit().map_err(|error| BackendEmitError(error.to_string()))
}

fn reject_float_ir(
    backend_module: &BackendModule,
    configuration: &NativeBackendConfiguration,
) -> Result<(), BackendEmitError> {
    if !configuration.verifies_no_float_ir() {
        return Ok(());
    }
    for function in &backend_module.functions {
        if function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    instruction,
                    BackendInstruction::FloatConstant { .. }
                        | BackendInstruction::FloatCompare { .. }
                        | BackendInstruction::FloatCast { .. }
                )
            })
        }) {
            return Err(BackendEmitError(format!(
                "native function `{}` contains floating-point IR instructions: shared float instruction",
                function.symbol.as_str()
            )));
        }
    }
    Ok(())
}
