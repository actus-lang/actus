use super::emission::EmittedObject;
use crate::codegen::{BackendEmitError, BackendSymbolBindings};
use crate::configuration::CompilerConfiguration;
use crate::modules::ModuleObjectPlan;

pub(super) fn emit_shared_scalar_units(
    object_plan: &ModuleObjectPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    let modules = super::shared_emission::lower_shared_modules(
        object_plan,
        symbol,
        configuration,
        bindings,
        &[],
        "Cranelift shared",
    )?;
    modules
        .into_iter()
        .map(|lowered| {
            let bytes = crate::codegen::cranelift::emit_contract_module_object(
                &lowered.module,
                configuration.native_backend(),
                configuration.target(),
            )?;
            Ok(EmittedObject {
                name: lowered.object_name,
                bytes,
                symbols: lowered
                    .module
                    .functions
                    .iter()
                    .map(|function| function.symbol.as_str().to_owned())
                    .collect(),
            })
        })
        .collect()
}
pub(super) fn emit_shared_specialized_units(
    object_plan: &ModuleObjectPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    let modules = super::shared_emission::lower_shared_modules(
        object_plan,
        symbol,
        configuration,
        bindings,
        generic_instances,
        "Cranelift shared",
    )?;
    modules
        .into_iter()
        .map(|lowered| {
            let bytes = crate::codegen::cranelift::emit_contract_module_object(
                &lowered.module,
                configuration.native_backend(),
                configuration.target(),
            )?;
            Ok(EmittedObject {
                name: lowered.object_name,
                bytes,
                symbols: lowered
                    .module
                    .functions
                    .iter()
                    .map(|function| function.symbol.as_str().to_owned())
                    .collect(),
            })
        })
        .collect()
}

// Legacy Cranelift compatibility remains isolated from shared emission.
