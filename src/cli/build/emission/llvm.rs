use super::EmittedObject;
use crate::codegen::{BackendEmitError, BackendSymbolBindings, SymbolIdentity, SymbolKind};
use crate::configuration::CompilerConfiguration;
use crate::modules::{ModuleCompilationPlan, ModuleObjectPlan};

pub(super) fn emit_object_llvm(
    plan: &ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    check_locations: Option<&crate::SharedSourceMap>,
) -> Result<EmittedObject, BackendEmitError> {
    let (generic_types, generic_instances) = super::shared::generic_inputs(plan, configuration)?;
    let object_plan = plan
        .object_plan_with_generic_types(&generic_types)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let bindings = if plan.has_friend_scope() {
        llvm_module_bindings_with_preferred_unique_sources(&object_plan, &generic_instances)?
    } else {
        llvm_module_bindings(&object_plan, &generic_instances)?
    };
    let program = super::shared::lower_program(
        &object_plan,
        symbol,
        configuration,
        &bindings,
        &generic_instances,
        "llvm",
        check_locations,
    )?;
    crate::codegen::reject_float_ir(
        &program.module,
        configuration.native_backend().verifies_no_float_ir(),
    )?;
    llvm_object(
        "root",
        &program.module,
        &program.local_symbols,
        configuration.target(),
        configuration.native_backend().position_independent(),
        configuration.native_backend().verifies_no_float_ir(),
        crate::codegen::llvm::ObjectEmissionOptions {
            optimization_level: crate::codegen::llvm::ObjectOptimizationLevel::from_configuration(
                configuration.native_backend().optimization_level(),
            ),
            ..crate::codegen::llvm::ObjectEmissionOptions::default()
        },
    )
}

fn llvm_object(
    name: &str,
    module: &crate::codegen::BackendModule,
    local_symbols: &std::collections::BTreeSet<String>,
    target: &crate::target::TargetSpec,
    position_independent: bool,
    verify_no_float: bool,
    options: crate::codegen::llvm::ObjectEmissionOptions,
) -> Result<EmittedObject, BackendEmitError> {
    let bytes = crate::codegen::llvm::emit_object_bytes_with_local_symbols(
        module,
        local_symbols,
        target,
        position_independent,
        verify_no_float,
        options,
    )?;
    let symbols =
        module.functions.iter().map(|function| function.symbol.as_str().to_owned()).collect();
    Ok(EmittedObject { name: name.to_owned(), bytes, symbols })
}

fn llvm_module_bindings(
    plan: &ModuleObjectPlan,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<BackendSymbolBindings, BackendEmitError> {
    llvm_module_bindings_with_policy(plan, generic_instances, false)
}

fn llvm_module_bindings_with_preferred_unique_sources(
    plan: &ModuleObjectPlan,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<BackendSymbolBindings, BackendEmitError> {
    llvm_module_bindings_with_policy(plan, generic_instances, true)
}

fn llvm_module_bindings_with_policy(
    plan: &ModuleObjectPlan,
    generic_instances: &[crate::semantic::GenericInstance],
    prefer_first_unique_source: bool,
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
            for instance in
                crate::codegen::generic_instances_of_verb(unit.program(), name, generic_instances)
            {
                let specialized_full = crate::codegen::specialized_generic_name(instance);
                let specialized = specialized_full
                    .rsplit("::")
                    .next()
                    .unwrap_or(specialized_full.as_str())
                    .to_owned();
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
    Ok(if prefer_first_unique_source {
        BackendSymbolBindings::new_prefer_first_unique_source(bindings)
    } else {
        BackendSymbolBindings::new(bindings)
    })
}
