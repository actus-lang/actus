use super::emission::EmittedObject;
use crate::codegen::{BackendEmitError, BackendSymbolBindings, SymbolIdentity, SymbolKind};
use crate::configuration::CompilerConfiguration;
use crate::modules::{ModuleCompilationPlan, ModuleObjectPlan};

pub(super) fn emit_objects_llvm(
    plan: &ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    let (generic_types, generic_instances) = llvm_generic_inputs(plan, configuration)?;
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
        "LLVM",
    )?;
    modules
        .into_iter()
        .enumerate()
        .map(|(index, lowered)| {
            let module = if index == 0 {
                crate::codegen::llvm::retain_reachable_root_functions(lowered.module, symbol)
            } else {
                lowered.module
            };
            llvm_object(&lowered.object_name, &module, configuration.target())
        })
        .collect()
}

fn llvm_generic_inputs(
    plan: &ModuleCompilationPlan,
    configuration: &CompilerConfiguration,
) -> Result<(Vec<crate::ast::TypeName>, Vec<crate::semantic::GenericInstance>), BackendEmitError> {
    let targeted_caller = crate::codegen::normalize_program(
        &crate::semantic::filter_program_for_target(plan.caller(), configuration.target()),
    );
    let caller_instances = crate::semantic::analyze(&targeted_caller)
        .map_err(|error| BackendEmitError(format!("semantic analysis failed: {error:?}")))?
        .generic_instances;
    let mut generic_instances = caller_instances;
    for unit in plan.units() {
        if !contains_generic_declaration(unit.implementation()) {
            continue;
        }
        let targeted_implementation =
            crate::codegen::normalize_program(&crate::semantic::filter_program_for_target(
                unit.implementation(),
                configuration.target(),
            ));
        let implementation_instances = crate::semantic::analyze(&targeted_implementation)
            .map_err(|error| BackendEmitError(format!("semantic analysis failed: {error:?}")))?
            .generic_instances;
        for instance in implementation_instances {
            if !generic_instances.iter().any(|known| {
                known.identity() == instance.identity() && known.caller == instance.caller
            }) {
                generic_instances.push(instance);
            }
        }
    }
    let expansion_program = crate::codegen::normalize_program(
        &crate::semantic::filter_program_for_target(&plan.implementation(), configuration.target()),
    );
    let generic_instances =
        crate::codegen::expand_generic_instances(&expansion_program, &generic_instances)?;
    let generic_types =
        generic_instances.iter().flat_map(|instance| instance.arguments.iter().cloned()).collect();
    Ok((generic_types, generic_instances))
}

fn contains_generic_declaration(program: &crate::ast::Program) -> bool {
    program.declarations.iter().any(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => !verb.generic_parameters.is_empty(),
        crate::ast::TopLevelDecl::ExternalVerb(verb) => !verb.generic_parameters.is_empty(),
        crate::ast::TopLevelDecl::Struct(definition) => !definition.generic_parameters.is_empty(),
        crate::ast::TopLevelDecl::Enum(definition) => !definition.generic_parameters.is_empty(),
        _ => false,
    })
}

fn llvm_object(
    name: &str,
    module: &crate::codegen::BackendModule,
    target: &crate::target::TargetSpec,
) -> Result<EmittedObject, BackendEmitError> {
    let bytes = crate::codegen::llvm::emit_object_bytes(module, target)?;
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
