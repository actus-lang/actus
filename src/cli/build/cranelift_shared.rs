use super::emission::{EmittedObject, declared_symbols};
use crate::codegen::{BackendEmitError, BackendSymbolBindings};
use crate::configuration::CompilerConfiguration;

pub(super) fn emit_shared_scalar_root(
    source: &crate::ast::Program,
    namespace: &str,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
) -> Result<EmittedObject, BackendEmitError> {
    let program = crate::codegen::normalize_program(source);
    let module = crate::codegen::lower_program_to_backend_module(
        namespace,
        &program,
        Some(symbol),
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
        name: "root".to_owned(),
        bytes,
        symbols: declared_symbols(source, namespace, symbol, bindings)?,
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
