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
    let bindings = llvm_module_bindings(&object_plan)?;
    let root = &object_plan.units()[0];
    let root_program = specialize_llvm_program(root.program(), configuration, &generic_instances)?;
    let root_module = crate::codegen::lower_program_to_backend_module(
        root.namespace().symbol_prefix(),
        &root_program,
        Some(symbol),
        Some(&bindings),
    )
    .map_err(|error| {
        let error = error.with_backend_context("llvm", configuration.target().triple().to_string());
        BackendEmitError(format!("LLVM root semantic lowering failed: {error}"))
    })?;
    let mut objects = vec![llvm_object("root", &root_module, configuration.target())?];
    for unit in &object_plan.units()[1..] {
        let program = specialize_llvm_program(unit.program(), configuration, &generic_instances)?;
        let module = crate::codegen::lower_program_to_backend_module(
            unit.namespace().symbol_prefix(),
            &program,
            None,
            Some(&bindings),
        )
        .map_err(|error| {
            let error =
                error.with_backend_context("llvm", configuration.target().triple().to_string());
            BackendEmitError(format!(
                "LLVM module `{}` semantic lowering failed: {error}",
                unit.namespace().module_path()
            ))
        })?;
        objects.push(llvm_object(
            unit.namespace().symbol_prefix(),
            &module,
            configuration.target(),
        )?);
    }
    Ok(objects)
}

#[cfg(feature = "cranelift")]
fn llvm_generic_inputs(
    plan: &ModuleCompilationPlan,
    configuration: &CompilerConfiguration,
) -> Result<(Vec<crate::ast::TypeName>, Vec<crate::semantic::GenericInstance>), BackendEmitError> {
    let targeted_caller = crate::codegen::normalize_program(
        &crate::semantic::filter_program_for_target(plan.caller(), configuration.target()),
    );
    let generic_instances = crate::semantic::analyze(&targeted_caller)
        .map_err(|error| BackendEmitError(format!("semantic analysis failed: {error:?}")))?
        .generic_instances;
    let generic_instances =
        crate::codegen::expand_generic_instances(&targeted_caller, &generic_instances)?;
    let generic_types =
        generic_instances.iter().flat_map(|instance| instance.arguments.iter().cloned()).collect();
    Ok((generic_types, generic_instances))
}

#[cfg(not(feature = "cranelift"))]
fn llvm_generic_inputs(
    _plan: &ModuleCompilationPlan,
    _configuration: &CompilerConfiguration,
) -> Result<(Vec<crate::ast::TypeName>, Vec<crate::semantic::GenericInstance>), BackendEmitError> {
    Ok((Vec::new(), Vec::new()))
}

#[cfg(feature = "cranelift")]
fn specialize_llvm_program(
    program: &crate::ast::Program,
    configuration: &CompilerConfiguration,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<crate::ast::Program, BackendEmitError> {
    let targeted = crate::semantic::filter_program_for_target(program, configuration.target());
    let normalized = crate::codegen::normalize_program(&targeted);
    crate::codegen::specialize_program(&normalized, generic_instances)
}

#[cfg(not(feature = "cranelift"))]
fn specialize_llvm_program(
    program: &crate::ast::Program,
    configuration: &CompilerConfiguration,
    _generic_instances: &[crate::semantic::GenericInstance],
) -> Result<crate::ast::Program, BackendEmitError> {
    let targeted = crate::semantic::filter_program_for_target(program, configuration.target());
    Ok(crate::codegen::normalize_program(&targeted))
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
        }
    }
    Ok(BackendSymbolBindings::new(bindings))
}
