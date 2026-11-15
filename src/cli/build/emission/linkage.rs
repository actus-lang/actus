//! Symbol linkage follows declaration reach (ADR-0112).
//!
//! A function keeps a global symbol only when something outside its object
//! needs it: the entry point, a public verb of an imported unit, or a symbol
//! another object of the program references (a call, a cleanup performance
//! bridge, or an external declaration). Every other function is local to its
//! object. Test plans reach private declarations across objects, so they keep
//! every symbol global.

use std::collections::BTreeSet;

use crate::codegen::{
    BackendEmitError, BackendInstruction, BackendModule, SymbolIdentity, SymbolKind,
};
use crate::modules::ModuleObjectPlan;

use super::shared::LoweredSharedModule;

pub(super) fn assign_local_symbols(
    modules: &mut [LoweredSharedModule],
    object_plan: &ModuleObjectPlan,
    entry: &str,
) -> Result<(), BackendEmitError> {
    if object_plan.friend_scope() {
        return Ok(());
    }
    let mut global = BTreeSet::from([entry.to_owned()]);
    for unit in object_plan.units().iter().skip(1) {
        for name in unit.public_verbs() {
            let symbol =
                SymbolIdentity::new(unit.namespace().symbol_prefix(), SymbolKind::Verb, name)
                    .map_err(|error| BackendEmitError(error.to_string()))?;
            global.insert(symbol.as_str().to_owned());
        }
    }
    for lowered in modules.iter() {
        let defined = defined_symbols(&lowered.module);
        global.extend(
            referenced_symbols(&lowered.module)
                .into_iter()
                .filter(|symbol| !defined.contains(symbol)),
        );
    }
    for lowered in modules.iter_mut() {
        lowered.local_symbols = lowered
            .module
            .functions
            .iter()
            .map(|function| function.symbol.as_str().to_owned())
            .filter(|symbol| !global.contains(symbol))
            .collect();
    }
    Ok(())
}

fn defined_symbols(module: &BackendModule) -> BTreeSet<String> {
    module.functions.iter().map(|function| function.symbol.as_str().to_owned()).collect()
}

/// Every symbol a module names: calls, cleanup performance bridges, and its
/// external declarations.
fn referenced_symbols(module: &BackendModule) -> BTreeSet<String> {
    let mut symbols = module
        .externals
        .iter()
        .map(|external| external.symbol.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    for function in &module.functions {
        for instruction in function.blocks.iter().flat_map(|block| block.instructions.iter()) {
            match instruction {
                BackendInstruction::Call { callee, .. } => {
                    symbols.insert(callee.as_str().to_owned());
                }
                BackendInstruction::Cleanup { operand: Some(operand), .. } => {
                    symbols.extend(
                        operand.performance_symbol.iter().map(|symbol| symbol.as_str().to_owned()),
                    );
                }
                _ => {}
            }
        }
        symbols.extend(
            function
                .cleanup
                .iter()
                .flat_map(|cleanup| cleanup.resolved_bridges.iter().flatten())
                .filter_map(|descriptor| descriptor.performance_symbol.as_ref())
                .map(|symbol| symbol.as_str().to_owned()),
        );
    }
    symbols
}
