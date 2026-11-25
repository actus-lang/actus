use std::collections::HashSet;

use crate::ast::{EnumPayload, GenericParam, Param, Program, ReturnType, TopLevelDecl, TypeName};
use crate::lexer::SourceSpan;

use super::super::types::ModuleError;
use super::super::unit::ModuleUnit;
use super::expression_checks;

pub(in crate::modules::aggregation) fn validate_import_visibility(
    program: &Program,
    module_path: &str,
    namespace: Option<&str>,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    if program
        .declarations
        .iter()
        .any(|declaration| matches!(declaration, TopLevelDecl::OpenSibling(_)))
    {
        return Ok(());
    }
    let private_verbs = private_names(unit, &["verb"]);
    // A role names a contract, not a value type, yet it appears wherever a type
    // name does (`dynamic R` parameters, bounds, `perform R for T`).
    let private_types = private_names(unit, &["struct", "pack", "enum", "role"]);
    let private_constants = private_names(unit, &["const"]);
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
                &private_constants,
                module_path,
                namespace,
                unit,
            )?;
        }
        check_signatures(declaration, &private_types, module_path, unit)?;
    }
    Ok(())
}

/// Rejects private types and roles of the imported unit in the importer's
/// declaration signatures: parameters, returns, fields, generic bounds and
/// `perform` headers.
fn check_signatures(
    declaration: &TopLevelDecl,
    private: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    let check = |ty: &TypeName| check_type_tree(ty, private, module_path, unit);
    let check_callable = |generics: &[GenericParam], params: &[Param], ret: Option<&ReturnType>| {
        generics.iter().try_for_each(|generic| check_generic(generic, &check))?;
        params.iter().try_for_each(|param| check(&param.ty))?;
        ret.map_or(Ok(()), |ret| check(&ret.ty))
    };
    match declaration {
        TopLevelDecl::Verb(verb) => {
            check_callable(&verb.generic_parameters, &verb.params, verb.return_type.as_ref())
        }
        TopLevelDecl::Struct(structure) => {
            structure
                .generic_parameters
                .iter()
                .try_for_each(|generic| check_generic(generic, &check))?;
            structure.fields.iter().try_for_each(|field| check(&field.ty))
        }
        TopLevelDecl::Enum(value) => {
            value
                .generic_parameters
                .iter()
                .try_for_each(|generic| check_generic(generic, &check))?;
            value.variants.iter().try_for_each(|variant| match &variant.payload {
                EnumPayload::Unit => Ok(()),
                EnumPayload::Tuple(types) => types.iter().try_for_each(&check),
                EnumPayload::Struct(fields) => fields.iter().try_for_each(|field| check(&field.ty)),
            })
        }
        TopLevelDecl::Perform(perform) => {
            reject_private(&perform.role_name, private, module_path, unit, perform.span)?;
            check(&perform.target)?;
            perform.methods.iter().try_for_each(|method| {
                check_callable(
                    &method.generic_parameters,
                    &method.params,
                    method.return_type.as_ref(),
                )
            })
        }
        _ => Ok(()),
    }
}

fn check_generic(
    generic: &GenericParam,
    check: &dyn Fn(&TypeName) -> Result<(), ModuleError>,
) -> Result<(), ModuleError> {
    generic.bound.iter().chain(&generic.bounds).try_for_each(check)
}

fn check_type_tree(
    ty: &TypeName,
    private: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    reject_private(&ty.name, private, module_path, unit, ty.span)?;
    ty.arguments
        .iter()
        .try_for_each(|argument| check_type_tree(argument, private, module_path, unit))
}

fn private_names(unit: &ModuleUnit, kinds: &[&str]) -> HashSet<String> {
    unit.implementation()
        .declarations
        .iter()
        // Internal import aliases (ADR-0110) name the unit's own verbs.
        .filter(|declaration| {
            !matches!(declaration, TopLevelDecl::ExternalVerb(verb) if verb.module_import)
        })
        .filter_map(|declaration| super::super::validation::export_identity(declaration))
        .filter(|(declaration_kind, name)| {
            kinds.contains(declaration_kind) && !unit.exports().contains(declaration_kind, name)
        })
        .map(|(_, name)| name)
        .collect()
}

pub(super) fn check_type_name(
    name: &str,
    private_types: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
    span: SourceSpan,
) -> Result<(), ModuleError> {
    reject_private(name, private_types, module_path, unit, span)
}

pub(super) fn reject_private(
    name: &str,
    private_names: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
    span: SourceSpan,
) -> Result<(), ModuleError> {
    if private_names.contains(name) {
        return Err(super::cause::private_access(unit, module_path, name, span));
    }
    Ok(())
}
