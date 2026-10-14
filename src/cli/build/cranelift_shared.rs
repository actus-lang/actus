use super::emission::{EmittedObject, declared_symbols};
use crate::codegen::{BackendEmitError, BackendSymbolBindings};
use crate::configuration::CompilerConfiguration;
use crate::modules::ModuleObjectPlan;

pub(super) fn supports_shared_scalar_units(
    object_plan: &ModuleObjectPlan,
    configuration: &CompilerConfiguration,
) -> bool {
    object_plan.units().iter().all(|unit| {
        let targeted =
            crate::semantic::filter_program_for_target(unit.program(), configuration.target());
        super::shared_candidate::supports_scalar_shared_lowering(&targeted)
    })
}

pub(super) fn supports_shared_specialized_units(
    object_plan: &ModuleObjectPlan,
    configuration: &CompilerConfiguration,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<bool, BackendEmitError> {
    if generic_instances.is_empty() {
        return Ok(false);
    }
    object_plan
        .units()
        .iter()
        .map(|unit| {
            let targeted = crate::codegen::normalize_program(
                &crate::semantic::filter_program_for_target(unit.program(), configuration.target()),
            );
            let specialized = crate::codegen::specialize_program(&targeted, generic_instances)?;
            Ok(super::shared_candidate::supports_scalar_shared_lowering(&specialized))
        })
        .collect::<Result<Vec<_>, BackendEmitError>>()
        .map(|results| results.into_iter().all(|supported| supported))
}
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
                symbols: declared_symbols(
                    &lowered.program,
                    &lowered.namespace,
                    if lowered.namespace == object_plan.units()[0].namespace().symbol_prefix() {
                        symbol
                    } else {
                        ""
                    },
                    bindings,
                )?,
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
                symbols: declared_symbols(
                    &lowered.program,
                    &lowered.namespace,
                    if lowered.namespace == object_plan.units()[0].namespace().symbol_prefix() {
                        symbol
                    } else {
                        ""
                    },
                    bindings,
                )?,
            })
        })
        .collect()
}

pub(super) fn emit_cranelift_root(
    program: &crate::ast::Program,
    namespace: &str,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
) -> Result<EmittedObject, BackendEmitError> {
    let bytes =
        crate::codegen::cranelift::emit_program_object_for_target_in_namespace_with_bindings(
            program,
            symbol,
            namespace,
            configuration.native_backend(),
            configuration.target(),
            bindings,
        )?;
    Ok(EmittedObject {
        name: "root".to_owned(),
        bytes,
        symbols: declared_symbols(program, namespace, symbol, bindings)?,
    })
}
// Legacy Cranelift compatibility remains isolated from shared emission.
