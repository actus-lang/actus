use std::sync::Arc;

use crate::session::{AnnotatedUnit, CompilationSession};

use super::super::registry::{DeclarationRegistry, ROOT_MODULE_PATH};
use super::super::unit::ModuleUnit;
use super::compilation::ModuleCompilationPlan;

/// Builds the plan's declaration registry and annotates every program in the
/// plan once (ADR-0108). With a session, each unit's annotation is a memoized
/// query shared by every plan of the same run that contains the same unit
/// revisions (ADR-0109).
pub(super) fn annotate_plan(
    plan: &mut ModuleCompilationPlan,
    importer_module_path: Option<&str>,
    session: Option<&CompilationSession>,
) {
    let registry = Arc::new(DeclarationRegistry::from_plan_with_session(plan, session));
    plan.registry = Arc::clone(&registry);
    let scope = importer_module_path.unwrap_or(ROOT_MODULE_PATH);
    registry.annotate_program_headers(scope, &mut plan.local);
    registry.annotate_program_headers(scope, &mut plan.caller);
    let Some(session) = session else {
        for unit in &mut plan.units {
            unit.annotate_types(&registry);
        }
        return;
    };
    // Conservative dependency fingerprint: every unit revision in the plan.
    let closure =
        plan.units.iter().map(|unit| unit.identity().source_key()).collect::<Vec<_>>().join("|");
    for unit in &mut plan.units {
        let key = annotated_unit_key(unit, &closure);
        // The unit is copied only on a miss; a hit reuses the published result.
        let annotated = session.annotated_unit(key, || annotate_unit(unit, &registry));
        unit.replace_with_annotated(&annotated);
    }
}

fn annotate_unit(unit: &ModuleUnit, registry: &DeclarationRegistry) -> AnnotatedUnit {
    let module_path = unit.identity().module_path();
    let mut implementation = unit.implementation().clone();
    registry.annotate_program_headers(module_path, &mut implementation);
    let sources = unit
        .sources()
        .iter()
        .map(|source| {
            let mut program = source.program().clone();
            registry.annotate_program_headers(module_path, &mut program);
            program
        })
        .collect();
    AnnotatedUnit { implementation, sources }
}

fn annotated_unit_key(unit: &ModuleUnit, closure: &str) -> String {
    format!(
        "{}::{}@{}#{}",
        unit.identity().package(),
        unit.identity().module_path(),
        unit.identity().source_key(),
        closure
    )
}
