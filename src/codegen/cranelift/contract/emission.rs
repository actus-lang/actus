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
    emit_contract_module_object_with_local_symbols(
        backend_module,
        &std::collections::BTreeSet::new(),
        configuration,
        target,
    )
}

/// Emits the module with `local_symbols` given local linkage (ADR-0112).
pub fn emit_contract_module_object_with_local_symbols(
    backend_module: &BackendModule,
    local_symbols: &std::collections::BTreeSet<String>,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    backend_module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    crate::codegen::reject_float_ir(backend_module, configuration.verifies_no_float_ir())?;
    let mut module = create_module(configuration, target)?;
    let declarations = declarations::declare_functions(&mut module, backend_module, local_symbols)?;
    for function in &backend_module.functions {
        functions::define_function(
            &mut module,
            function,
            &declarations,
            &backend_module.runtime_checks,
        )?;
    }
    module.finish().emit().map_err(|error| BackendEmitError(error.to_string()))
}
