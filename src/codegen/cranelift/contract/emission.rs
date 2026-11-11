use super::super::native::BackendEmitError;
use super::super::object::create_module;
use super::{declarations, functions};
use crate::codegen::BackendModule;
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

pub fn emit_contract_module_object(
    backend_module: &BackendModule,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    backend_module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    crate::codegen::reject_float_ir(backend_module, configuration.verifies_no_float_ir())?;
    let mut module = create_module(configuration, target)?;
    let declarations = declarations::declare_functions(&mut module, backend_module)?;
    for function in &backend_module.functions {
        functions::define_function(&mut module, function, &declarations)?;
    }
    module.finish().emit().map_err(|error| BackendEmitError(error.to_string()))
}
