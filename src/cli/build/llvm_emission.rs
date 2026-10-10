use super::emission::EmittedObject;
use crate::codegen::{BackendEmitError, BackendSymbolBindings, SymbolIdentity, SymbolKind};
use crate::configuration::CompilerConfiguration;
use crate::modules::{ModuleCompilationPlan, ModuleObjectPlan};

pub(super) fn emit_objects_llvm(
    plan: &ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    let object_plan = plan.object_plan().map_err(|error| BackendEmitError(error.to_string()))?;
    let bindings = llvm_module_bindings(&object_plan)?;
    let root = &object_plan.units()[0];
    let root_program = crate::codegen::normalize_program(root.program());
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
        let program = crate::codegen::normalize_program(unit.program());
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
