use crate::codegen::{BackendEmitError, BackendModule, BackendSymbolBindings};
use crate::configuration::CompilerConfiguration;
use crate::modules::ModuleObjectPlan;

pub(super) fn retain_reachable_root_functions(
    mut module: crate::codegen::BackendModule,
    entry_symbol: &str,
) -> crate::codegen::BackendModule {
    let functions = module
        .functions
        .iter()
        .map(|function| (function.symbol.as_str().to_owned(), function))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut reachable = std::collections::BTreeSet::new();
    let mut pending = std::collections::VecDeque::from([entry_symbol.to_owned()]);
    while let Some(symbol) = pending.pop_front() {
        if !reachable.insert(symbol.clone()) {
            continue;
        }
        let Some(function) = functions.get(&symbol) else { continue };
        for block in &function.blocks {
            for instruction in &block.instructions {
                let crate::codegen::BackendInstruction::Call { callee, .. } = instruction else {
                    continue;
                };
                let callee = callee.as_str();
                if functions.contains_key(callee) {
                    pending.push_back(callee.to_owned());
                }
            }
        }
        for cleanup in &function.cleanup {
            for descriptor in cleanup.resolved_bridges.iter().flatten() {
                let Some(symbol) = descriptor.performance_symbol.as_ref() else { continue };
                let symbol = symbol.as_str();
                if functions.contains_key(symbol) {
                    pending.push_back(symbol.to_owned());
                }
            }
        }
    }
    module.functions.retain(|function| reachable.contains(function.symbol.as_str()));
    module
}

pub(super) struct LoweredSharedModule {
    pub(super) namespace: String,
    pub(super) object_name: String,
    pub(super) program: crate::ast::Program,
    pub(super) module: BackendModule,
}

pub(super) fn generic_inputs(
    plan: &crate::modules::ModuleCompilationPlan,
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
            let module =
                if index == 0 { retain_reachable_root_functions(module, symbol) } else { module };
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
