use super::EmittedObject;
use crate::codegen::{BackendEmitError, BackendSymbolBindings};
use crate::configuration::CompilerConfiguration;
use crate::modules::ModuleObjectPlan;

/// Emits the program as one Cranelift object (ADR-0113).
pub(super) fn emit_program_object(
    object_plan: &ModuleObjectPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<EmittedObject, BackendEmitError> {
    let program = super::shared::lower_program(
        object_plan,
        symbol,
        configuration,
        bindings,
        generic_instances,
        "Cranelift shared",
    )?;
    let bytes = crate::codegen::cranelift::emit_contract_module_object_with_local_symbols(
        &program.module,
        &program.local_symbols,
        configuration.native_backend(),
        configuration.target(),
    )?;
    Ok(EmittedObject {
        name: "root".to_owned(),
        bytes,
        symbols: program
            .module
            .functions
            .iter()
            .map(|function| function.symbol.as_str().to_owned())
            .collect(),
    })
}
