use crate::codegen::{BackendEmitError, BackendModule, BackendSymbolBindings};
use crate::configuration::CompilerConfiguration;
use crate::modules::ModuleObjectPlan;

pub(super) fn retain_reachable_root_functions(
    module: &mut crate::codegen::BackendModule,
    entry_symbol: &str,
    required_symbols: &std::collections::BTreeSet<String>,
) {
    let functions = module
        .functions
        .iter()
        .map(|function| (function.symbol.as_str().to_owned(), function))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut reachable = std::collections::BTreeSet::new();
    let mut pending = std::collections::VecDeque::from([entry_symbol.to_owned()]);
    pending.extend(required_symbols.iter().cloned());
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
    let object_plan = plan
        .object_plan()
        .map_err(|error| BackendEmitError(format!("module object planning failed: {error:?}")))?;
    let mut unique_instances = Vec::new();
    for unit in object_plan.units() {
        let expansion_program = crate::codegen::normalize_program(
            &crate::semantic::filter_program_for_target(unit.program(), configuration.target()),
        );
        let generic_instances = crate::semantic::analyze(&expansion_program)
            .map_err(|error| BackendEmitError(format!("semantic analysis failed: {error:?}")))?
            .generic_instances;
        let expanded_instances =
            crate::codegen::expand_generic_instances(&expansion_program, &generic_instances)?;
        for instance in expanded_instances {
            if !unique_instances.iter().any(|known: &crate::semantic::GenericInstance| {
                known.identity() == instance.identity() && known.caller == instance.caller
            }) {
                unique_instances.push(instance);
            }
        }
    }
    let generic_types =
        unique_instances.iter().flat_map(|instance| instance.arguments.iter().cloned()).collect();
    Ok((generic_types, unique_instances))
}

pub(super) fn lower_shared_modules(
    object_plan: &ModuleObjectPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &BackendSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
    backend: &str,
) -> Result<Vec<LoweredSharedModule>, BackendEmitError> {
    let context =
        SharedLoweringContext { symbol, configuration, bindings, generic_instances, backend };
    let mut modules = object_plan
        .units()
        .iter()
        .enumerate()
        .map(|(index, _)| lower_unit_module(object_plan, index, &context))
        .collect::<Result<Vec<_>, _>>()?;
    let required_symbols = required_performance_symbols(&modules);
    if let Some(root) = modules.first_mut() {
        retain_reachable_root_functions(&mut root.module, symbol, &required_symbols);
    }
    Ok(modules)
}

struct SharedLoweringContext<'a> {
    symbol: &'a str,
    configuration: &'a CompilerConfiguration,
    bindings: &'a BackendSymbolBindings,
    generic_instances: &'a [crate::semantic::GenericInstance],
    backend: &'a str,
}

fn lower_unit_module(
    object_plan: &ModuleObjectPlan,
    index: usize,
    context: &SharedLoweringContext<'_>,
) -> Result<LoweredSharedModule, BackendEmitError> {
    let unit = &object_plan.units()[index];
    let targeted = crate::codegen::normalize_program(&crate::semantic::filter_program_for_target(
        unit.program(),
        context.configuration.target(),
    ));
    let program = crate::codegen::specialize_program(&targeted, context.generic_instances)?;
    let performance_context = (index != 0).then(|| {
        (object_plan.units()[0].namespace().symbol_prefix(), object_plan.units()[0].program())
    });
    let module =
        lower_unit_backend_module(object_plan, index, &program, performance_context, context)?;
    Ok(LoweredSharedModule {
        namespace: unit.namespace().symbol_prefix().to_owned(),
        object_name: unit_object_name(unit, index),
        program,
        module,
    })
}

fn lower_unit_backend_module(
    object_plan: &ModuleObjectPlan,
    index: usize,
    program: &crate::ast::Program,
    performance_context: Option<(&str, &crate::ast::Program)>,
    context: &SharedLoweringContext<'_>,
) -> Result<crate::codegen::BackendModule, BackendEmitError> {
    let unit = &object_plan.units()[index];
    crate::codegen::lower_program_to_backend_module_for_target_with_context(
        unit.namespace().symbol_prefix(),
        program,
        (index == 0).then_some(context.symbol),
        Some(context.bindings),
        context.configuration.target(),
        performance_context,
    )
    .map_err(|error| {
        let error = error.with_backend_context(
            context.backend,
            context.configuration.target().triple().to_string(),
        );
        let location = if index == 0 {
            "root semantic lowering".to_owned()
        } else {
            format!("module `{}` semantic lowering", unit.namespace().module_path())
        };
        BackendEmitError(format!("{} {} failed: {error}", context.backend, location))
    })
}

fn unit_object_name(unit: &crate::modules::ModuleObjectUnit, index: usize) -> String {
    if index == 0 { "root".to_owned() } else { unit.namespace().symbol_prefix().to_owned() }
}

fn required_performance_symbols(
    modules: &[LoweredSharedModule],
) -> std::collections::BTreeSet<String> {
    modules
        .iter()
        .skip(1)
        .flat_map(|lowered| lowered.module.functions.iter())
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction {
            crate::codegen::BackendInstruction::Call { callee, .. }
                if callee.kind() == crate::codegen::SymbolKind::Performance =>
            {
                Some(callee.as_str().to_owned())
            }
            _ => None,
        })
        .collect()
}
