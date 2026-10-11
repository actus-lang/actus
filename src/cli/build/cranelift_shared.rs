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

pub(super) fn emit_shared_scalar_units(
    object_plan: &ModuleObjectPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    let mut objects = Vec::with_capacity(object_plan.units().len());
    for (index, unit) in object_plan.units().iter().enumerate() {
        let targeted =
            crate::semantic::filter_program_for_target(unit.program(), configuration.target());
        let object = if index == 0 {
            emit_shared_scalar_root(
                &targeted,
                unit.namespace().symbol_prefix(),
                symbol,
                configuration,
                bindings,
            )?
        } else {
            emit_shared_scalar_unit(
                &targeted,
                unit.namespace().symbol_prefix(),
                None,
                unit.namespace().symbol_prefix(),
                configuration,
                bindings,
            )?
        };
        objects.push(object);
    }
    Ok(objects)
}

pub(super) fn emit_shared_scalar_root(
    source: &crate::ast::Program,
    namespace: &str,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
) -> Result<EmittedObject, BackendEmitError> {
    emit_shared_scalar_unit(source, namespace, Some(symbol), "root", configuration, bindings)
}

pub(super) fn emit_shared_scalar_unit(
    source: &crate::ast::Program,
    namespace: &str,
    entry_symbol: Option<&str>,
    object_name: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
) -> Result<EmittedObject, BackendEmitError> {
    let program = crate::codegen::normalize_program(source);
    let module = crate::codegen::lower_program_to_backend_module(
        namespace,
        &program,
        entry_symbol,
        Some(bindings),
    )
    .map_err(|error| {
        BackendEmitError(format!(
            "Cranelift shared semantic lowering failed: {}",
            error.with_backend_context("cranelift", configuration.target().triple().to_string())
        ))
    })?;
    let bytes = crate::codegen::cranelift::emit_contract_module_object(
        &module,
        configuration.native_backend(),
        configuration.target(),
    )?;
    Ok(EmittedObject {
        name: object_name.to_owned(),
        bytes,
        symbols: declared_symbols(source, namespace, entry_symbol.unwrap_or(""), bindings)?,
    })
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
