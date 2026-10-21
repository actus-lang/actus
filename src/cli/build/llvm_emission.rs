use super::emission::EmittedObject;
use crate::codegen::{BackendEmitError, BackendSymbolBindings, SymbolIdentity, SymbolKind};
use crate::configuration::CompilerConfiguration;
use crate::modules::{ModuleCompilationPlan, ModuleObjectPlan};

pub(super) fn emit_objects_llvm(
    plan: &ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    let (generic_types, generic_instances) =
        super::shared_emission::generic_inputs(plan, configuration)?;
    let object_plan = plan
        .object_plan_with_generic_types(&generic_types)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let bindings = llvm_module_bindings(&object_plan, &generic_instances)?;
    let modules = super::shared_emission::lower_shared_modules(
        &object_plan,
        symbol,
        configuration,
        &bindings,
        &generic_instances,
        "llvm",
    )?;
    modules
        .into_iter()
        .map(|lowered| {
            llvm_object(
                &lowered.object_name,
                &lowered.module,
                configuration.target(),
                configuration.native_backend().position_independent(),
            )
        })
        .collect()
}

fn llvm_object(
    name: &str,
    module: &crate::codegen::BackendModule,
    target: &crate::target::TargetSpec,
    position_independent: bool,
) -> Result<EmittedObject, BackendEmitError> {
    let bytes = crate::codegen::llvm::emit_object_bytes_with_configuration(
        module,
        target,
        position_independent,
    )?;
    let symbols =
        module.functions.iter().map(|function| function.symbol.as_str().to_owned()).collect();
    Ok(EmittedObject { name: name.to_owned(), bytes, symbols })
}

fn llvm_module_bindings(
    plan: &ModuleObjectPlan,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<BackendSymbolBindings, BackendEmitError> {
    let mut bindings = Vec::new();
    for unit in &plan.units()[1..] {
        for name in unit.exported_verbs() {
            let symbol =
                SymbolIdentity::new(unit.namespace().symbol_prefix(), SymbolKind::Verb, name)
                    .map_err(|error| BackendEmitError(error.to_string()))?;
            bindings.push((
                unit.namespace().symbol_prefix().to_owned(),
                name.clone(),
                symbol.as_str().to_owned(),
            ));
            for instance in generic_instances.iter().filter(|instance| &instance.name == name) {
                let specialized = crate::codegen::specialized_generic_name(instance);
                let symbol = SymbolIdentity::new(
                    unit.namespace().symbol_prefix(),
                    SymbolKind::Verb,
                    &specialized,
                )
                .map_err(|error| BackendEmitError(error.to_string()))?;
                bindings.push((
                    unit.namespace().symbol_prefix().to_owned(),
                    specialized,
                    symbol.as_str().to_owned(),
                ));
            }
        }
    }
    Ok(BackendSymbolBindings::new(bindings))
}
