use std::collections::HashSet;

use crate::ast::{Program, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::types::ModuleError;
use super::unit::ModuleUnit;

mod expression_checks;

pub(super) fn validate_import_visibility(
    program: &Program,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    let private_verbs = private_names(unit, "verb");
    let private_types = private_names(unit, "struct");
    for declaration in &program.declarations {
        let block = match declaration {
            TopLevelDecl::Verb(verb) => Some(&verb.body),
            TopLevelDecl::Perform(perform) => perform.methods.first().map(|method| &method.body),
            _ => None,
        };
        if let Some(block) = block {
            expression_checks::check_block(
                block,
                &private_verbs,
                &private_types,
                module_path,
                unit,
            )?;
        }
        if let TopLevelDecl::Struct(structure) = declaration {
            for field in &structure.fields {
                check_type_name(&field.ty.name, &private_types, module_path, unit, field.span)?;
            }
        }
    }
    Ok(())
}

fn private_names(unit: &ModuleUnit, kind: &str) -> HashSet<String> {
    unit.implementation()
        .declarations
        .iter()
        .filter_map(|declaration| super::validation::export_identity(declaration))
        .filter(|(declaration_kind, name)| {
            *declaration_kind == kind && !unit.exports().contains(declaration_kind, name)
        })
        .map(|(_, name)| name)
        .collect()
}

fn check_type_name(
    name: &str,
    private_types: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
    span: SourceSpan,
) -> Result<(), ModuleError> {
    reject_private(name, private_types, module_path, unit, span)
}

fn reject_private(
    name: &str,
    private_names: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
    span: SourceSpan,
) -> Result<(), ModuleError> {
    if private_names.contains(name) {
        return Err(ModuleError::PrivateDeclarationAccess {
            module: module_path.to_owned(),
            symbol: name.to_owned(),
            facade: unit.facade().to_owned(),
            span,
        });
    }
    Ok(())
}
