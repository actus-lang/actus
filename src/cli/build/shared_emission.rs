use crate::codegen::{BackendEmitError, BackendModule, BackendSymbolBindings};
use crate::configuration::CompilerConfiguration;
use crate::modules::ModuleObjectPlan;

pub(super) struct LoweredSharedModule {
    pub(super) namespace: String,
    pub(super) object_name: String,
    pub(super) program: crate::ast::Program,
    pub(super) module: BackendModule,
}

pub(super) fn lower_shared_modules(
    object_plan: &ModuleObjectPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
    backend: &str,
) -> Result<Vec<LoweredSharedModule>, BackendEmitError> {
    object_plan
        .units()
        .iter()
        .enumerate()
        .map(|(index, unit)| {
            let targeted = crate::codegen::normalize_program(
                &crate::semantic::filter_program_for_target(unit.program(), configuration.target()),
            );
            let program = crate::codegen::specialize_program(&targeted, generic_instances)?;
            let entry_symbol = (index == 0).then_some(symbol);
            let module = crate::codegen::lower_program_to_backend_module_for_target(
                unit.namespace().symbol_prefix(),
                &program,
                entry_symbol,
                Some(bindings),
                configuration.target(),
            )
            .map_err(|error| {
                let error = error
                    .with_backend_context(backend, configuration.target().triple().to_string());
                let location = if index == 0 {
                    "root semantic lowering".to_owned()
                } else {
                    format!("module `{}` semantic lowering", unit.namespace().module_path())
                };
                BackendEmitError(format!("{backend} {location} failed: {error}"))
            })?;
            Ok(LoweredSharedModule {
                namespace: unit.namespace().symbol_prefix().to_owned(),
                object_name: if index == 0 {
                    "root".to_owned()
                } else {
                    unit.namespace().symbol_prefix().to_owned()
                },
                program,
                module,
            })
        })
        .collect()
}
