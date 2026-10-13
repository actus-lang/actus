use std::collections::BTreeSet;

use crate::ast::Program;

use super::{scalar_option_type, scalar_type};

pub(super) fn unit_structs(program: &Program) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    loop {
        let previous_len = names.len();
        for declaration in &program.declarations {
            let crate::ast::TopLevelDecl::Struct(definition) = declaration else { continue };
            if (supported_struct(definition, &names) || supported_value_struct(definition, &names))
                && names.insert(definition.name.clone())
            {}
        }
        if names.len() == previous_len {
            return names;
        }
    }
}

fn supported_struct(
    definition: &crate::ast::StructDef,
    aggregate_types: &BTreeSet<String>,
) -> bool {
    definition.generic_parameters.is_empty()
        && definition.fields.iter().all(|field| {
            (matches!(field.role, crate::ast::StructFieldRole::Erg)
                && (scalar_type(&field.ty) || aggregate_types.contains(&field.ty.name)))
                || (matches!(field.role, crate::ast::StructFieldRole::Abs)
                    && scalar_option_type(&field.ty))
        })
}

fn supported_value_struct(
    definition: &crate::ast::StructDef,
    aggregate_types: &BTreeSet<String>,
) -> bool {
    definition.generic_parameters.is_empty()
        && definition.fields.iter().all(|field| {
            (matches!(field.role, crate::ast::StructFieldRole::Value)
                && (scalar_type(&field.ty) || aggregate_types.contains(&field.ty.name)))
                || (matches!(field.role, crate::ast::StructFieldRole::Abs)
                    && scalar_option_type(&field.ty))
        })
}

pub(super) fn supported_perform(
    perform: &crate::ast::PerformDecl,
    program: &Program,
    aggregate_types: &BTreeSet<String>,
) -> bool {
    perform.target.arguments.is_empty()
        && program.declarations.iter().any(|declaration| {
            matches!(
                declaration,
                crate::ast::TopLevelDecl::Struct(definition)
                    if definition.name == perform.target.name
                        && supported_struct(definition, aggregate_types)
            )
        })
}
