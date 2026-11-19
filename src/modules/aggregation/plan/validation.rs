use std::collections::HashMap;

use crate::ast::{Program, TopLevelDecl};
use crate::target::TargetSpec;

use super::super::collision::{ExternalContract, external_contract};
use super::super::registry::ROOT_MODULE_PATH;
use super::super::types::ModuleError;
use super::super::unit::ModuleUnit;
use super::super::validation::{
    declarations_are_identical_without_spans, export_identity, target_scopes_are_disjoint,
    target_selector,
};

/// Validates source programs after target filtering and normalization, including
/// generated serialization verbs, but before generic specialization. The
/// caller, object units, and imported units with the root performance context
/// are checked here; later semantic failures are compiler invariants.
pub(crate) fn validate_compilation_plan(
    plan: &super::compilation::ModuleCompilationPlan,
    target: &TargetSpec,
) -> Result<(), ModuleError> {
    let caller = filter_program(plan.caller(), target)?;
    validate_program(&caller)?;
    let object_plan = plan.object_plan()?;
    let root_context = object_plan
        .units()
        .first()
        .map(|unit| filter_program(unit.program(), target))
        .transpose()?;
    for (index, unit) in object_plan.units().iter().enumerate() {
        let targeted = filter_program(unit.program(), target)?;
        validate_program(&targeted)?;
        if index != 0 {
            let contextual = super::performance_context::program_with_performance_context(
                &targeted,
                root_context.as_ref().expect("object plan has a root unit"),
            );
            validate_program(&contextual)?;
        }
    }
    Ok(())
}

fn filter_program(program: &Program, target: &TargetSpec) -> Result<Program, ModuleError> {
    crate::semantic::filter_program_for_target_checked(program, target)
        .map_err(|error| ModuleError::Semantic(Box::new(error)))
}

/// The plan already annotated `program` (ADR-0108); normalization preserves
/// the annotation, so validation analyzes it without resolving again.
fn validate_program(program: &Program) -> Result<(), ModuleError> {
    let normalized = crate::codegen::normalize_program(program);
    crate::semantic::analyze_resolved(&normalized)
        .map(|_| ())
        .map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub(super) fn validate_symbol_collisions(
    local_declarations: &[TopLevelDecl],
    units: &[ModuleUnit],
) -> Result<(), ModuleError> {
    let mut symbols = HashMap::new();
    record_symbols(ROOT_MODULE_PATH, local_declarations, &mut symbols)?;
    for unit in units {
        record_symbols(
            unit.identity().module_path(),
            &unit.implementation().declarations,
            &mut symbols,
        )?;
    }
    Ok(())
}

fn record_symbols(
    module: &str,
    declarations: &[TopLevelDecl],
    symbols: &mut HashMap<(String, String), Vec<SymbolRecord>>,
) -> Result<(), ModuleError> {
    for declaration in declarations {
        let Some((kind, name)) = export_identity(declaration) else { continue };
        let key = (kind.to_owned(), name.clone());
        let record = SymbolRecord {
            module: module.to_owned(),
            external_contract: external_contract(declaration),
            declaration: declaration.clone(),
        };
        let records = symbols.entry(key).or_default();
        for first in records.iter() {
            if first.module == module
                || target_scopes_are_disjoint(
                    target_selector(&first.declaration),
                    target_selector(&record.declaration),
                )
            {
                continue;
            }
            if declarations_are_identical_without_spans(&first.declaration, declaration) {
                continue;
            }
            if first.external_contract.is_some()
                && first.external_contract == record.external_contract
            {
                continue;
            }
            return Err(ModuleError::SymbolCollision {
                symbol: if record.external_contract.is_some() {
                    format!("native symbol `{name}`")
                } else {
                    format!("{kind} `{name}`")
                },
                first_module: first.module.clone(),
                second_module: module.to_owned(),
            });
        }
        records.push(record);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SymbolRecord {
    module: String,
    external_contract: Option<ExternalContract>,
    declaration: TopLevelDecl,
}
