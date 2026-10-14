use crate::ast::{Program, TopLevelDecl};
use crate::modules::{ModuleObjectPlan, ModuleObjectUnit};
use std::collections::HashSet;

pub(super) fn caller_definitions(
    object_plan: &ModuleObjectPlan,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Vec<TopLevelDecl> {
    let generic_targets = generic_instances
        .iter()
        .flat_map(|instance| instance.arguments.iter())
        .map(|argument| argument.canonical_key())
        .collect::<std::collections::HashSet<_>>();
    let private_types = object_plan.units()[0]
        .program()
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Struct(definition) if !definition.is_open => {
                Some(definition.name.clone())
            }
            TopLevelDecl::Enum(definition) if !definition.is_open => Some(definition.name.clone()),
            TopLevelDecl::Pack(definition) if !definition.is_open => Some(definition.name.clone()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    object_plan.units()[0]
        .program()
        .declarations
        .iter()
        .filter(|declaration| match declaration {
            TopLevelDecl::Struct(_) | TopLevelDecl::Enum(_) | TopLevelDecl::Pack(_) => true,
            TopLevelDecl::Perform(perform) => {
                generic_targets.contains(&perform.target.canonical_key())
                    && (private_types.contains(&perform.target.name)
                        || crate::ast::primitive_type(&perform.target.name).is_some()
                        || crate::ast::lookup_builtin_type(&perform.target.name).is_some())
            }
            _ => false,
        })
        .cloned()
        .collect()
}

pub(super) fn append_caller_definitions(
    module: &mut Program,
    caller_definitions: &[TopLevelDecl],
    include_performances: bool,
) {
    let roles = module
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Role(role) => Some(role.name.clone()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    let existing = module
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Struct(definition) => Some(definition.name.clone()),
            TopLevelDecl::Enum(definition) => Some(definition.name.clone()),
            TopLevelDecl::Pack(definition) => Some(definition.name.clone()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    let caller_types = caller_definitions
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Struct(definition) if !definition.is_open => {
                Some(definition.name.clone())
            }
            TopLevelDecl::Enum(definition) if !definition.is_open => Some(definition.name.clone()),
            TopLevelDecl::Pack(definition) if !definition.is_open => Some(definition.name.clone()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    module.declarations.extend(
        caller_definitions
            .iter()
            .filter(|declaration| {
                let name = match declaration {
                    TopLevelDecl::Struct(definition) => definition.name.as_str(),
                    TopLevelDecl::Enum(definition) => definition.name.as_str(),
                    TopLevelDecl::Pack(definition) => definition.name.as_str(),
                    TopLevelDecl::Perform(perform) => {
                        return include_performances
                            && roles.contains(&perform.role_name)
                            && !existing.contains(&perform.target.name)
                            && (caller_types.contains(&perform.target.name)
                                || crate::ast::primitive_type(&perform.target.name).is_some()
                                || crate::ast::lookup_builtin_type(&perform.target.name)
                                    .is_some());
                    }
                    _ => return false,
                };
                !existing.contains(name)
            })
            .cloned(),
    );
}

pub(super) fn requires_caller_context(
    unit: &ModuleObjectUnit,
    generic_instances: &[crate::semantic::GenericInstance],
    caller_definitions: &[TopLevelDecl],
) -> bool {
    let private_types = caller_definitions
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Struct(definition) if !definition.is_open => {
                Some(definition.name.as_str())
            }
            TopLevelDecl::Enum(definition) if !definition.is_open => Some(definition.name.as_str()),
            TopLevelDecl::Pack(definition) if !definition.is_open => Some(definition.name.as_str()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    unit.program().declarations.iter().any(|declaration| {
        let TopLevelDecl::Verb(verb) = declaration else { return false };
        !verb.generic_parameters.is_empty()
            && generic_instances.iter().any(|instance| {
                instance.name == verb.name
                    && instance.arguments.iter().any(|argument| {
                        private_types.contains(argument.name.as_str())
                            || crate::ast::primitive_type(&argument.name).is_some()
                            || crate::ast::lookup_builtin_type(&argument.name).is_some()
                    })
            })
    })
}

pub(super) fn roots_for_unit(
    unit: &ModuleObjectUnit,
    reachable_module_roots: &HashSet<String>,
) -> Vec<String> {
    if !unit.namespace().module_path().starts_with("std::wire") {
        return unit.exported_verbs().to_vec();
    }
    if unit.program().declarations.iter().all(|declaration| {
        !matches!(
            declaration,
            TopLevelDecl::Verb(verb) if reachable_module_roots.contains(&verb.name)
        )
    }) {
        return Vec::new();
    }
    unit.program()
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => Some(verb.name.clone()),
            _ => None,
        })
        .collect()
}
