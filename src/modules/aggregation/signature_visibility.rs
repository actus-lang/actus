use crate::ast::{
    EnumPayload, GenericParam, Param, PerformDecl, ReturnType, RoleDecl, TopLevelDecl, TypeName,
};
use std::collections::HashSet;

use super::types::{ModuleError, ModuleExports};
use super::unit::ModuleUnit;
use super::validation::export_identity;

struct PrivateNames {
    declarations: HashSet<String>,
    roles: HashSet<String>,
}

pub(super) fn validate_exported_signatures(unit: &ModuleUnit) -> Result<(), ModuleError> {
    let private = collect_private_names(unit);
    for declaration in &unit.implementation().declarations {
        if !is_exported(declaration, unit.exports()) {
            continue;
        }
        validate_declaration(declaration, &private, unit)?;
    }
    Ok(())
}

fn collect_private_names(unit: &ModuleUnit) -> PrivateNames {
    let mut private = PrivateNames { declarations: HashSet::new(), roles: HashSet::new() };
    for declaration in &unit.implementation().declarations {
        let Some((kind, name)) = export_identity(declaration) else { continue };
        if unit.exports().contains(kind, &name) {
            continue;
        }
        if kind == "role" {
            private.roles.insert(name);
        } else if matches!(kind, "struct" | "pack" | "enum") {
            private.declarations.insert(name);
        }
    }
    private
}

fn is_exported(declaration: &TopLevelDecl, exports: &ModuleExports) -> bool {
    let Some((kind, name)) = export_identity(declaration) else { return false };
    exports.contains(kind, &name)
}

fn validate_declaration(
    declaration: &TopLevelDecl,
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    match declaration {
        TopLevelDecl::Verb(value) => validate_callable(
            &value.generic_parameters,
            &value.params,
            value.return_type.as_ref(),
            private,
            unit,
        ),
        TopLevelDecl::ExternalVerb(value) => validate_callable(
            &value.generic_parameters,
            &value.params,
            value.return_type.as_ref(),
            private,
            unit,
        ),
        TopLevelDecl::Struct(value) => {
            validate_generics(&value.generic_parameters, private, unit)?;
            for field in &value.fields {
                validate_type(&field.ty, private, unit)?;
            }
            Ok(())
        }
        TopLevelDecl::Pack(value) => {
            validate_type(&value.storage, private, unit)?;
            for field in &value.fields {
                validate_type(&field.ty, private, unit)?;
            }
            Ok(())
        }
        TopLevelDecl::Enum(value) => {
            validate_generics(&value.generic_parameters, private, unit)?;
            for variant in &value.variants {
                match &variant.payload {
                    EnumPayload::Unit => {}
                    EnumPayload::Tuple(types) => {
                        for ty in types {
                            validate_type(ty, private, unit)?;
                        }
                    }
                    EnumPayload::Struct(fields) => {
                        for field in fields {
                            validate_type(&field.ty, private, unit)?;
                        }
                    }
                }
            }
            Ok(())
        }
        TopLevelDecl::Role(value) => validate_role(value, private, unit),
        TopLevelDecl::Perform(value) => validate_perform(value, private, unit),
        TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_) => Ok(()),
    }
}

fn validate_callable(
    generics: &[GenericParam],
    params: &[Param],
    return_type: Option<&ReturnType>,
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    validate_generics(generics, private, unit)?;
    for param in params {
        validate_type(&param.ty, private, unit)?;
    }
    if let Some(return_type) = return_type {
        validate_type(&return_type.ty, private, unit)?;
    }
    Ok(())
}

fn validate_generics(
    generics: &[GenericParam],
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    for generic in generics {
        if let Some(bound) = &generic.bound {
            validate_bound(bound, private, unit)?;
        }
        for bound in &generic.bounds {
            validate_bound(bound, private, unit)?;
        }
    }
    Ok(())
}

fn validate_bound(
    bound: &TypeName,
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    reject_private_role(&bound.name, bound.span, private, unit)?;
    validate_type(bound, private, unit)
}

fn validate_role(
    role: &RoleDecl,
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    for method in &role.methods {
        validate_callable(&[], &method.params, method.return_type.as_ref(), private, unit)?;
    }
    Ok(())
}

fn validate_perform(
    perform: &PerformDecl,
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    reject_private_role(&perform.role_name, perform.span, private, unit)?;
    validate_type(&perform.target, private, unit)?;
    for method in &perform.methods {
        validate_callable(
            &method.generic_parameters,
            &method.params,
            method.return_type.as_ref(),
            private,
            unit,
        )?;
    }
    Ok(())
}

fn validate_type(
    type_name: &TypeName,
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    if private.declarations.contains(&type_name.name) {
        return private_error(&type_name.name, type_name.span, unit);
    }
    for argument in &type_name.arguments {
        validate_type(argument, private, unit)?;
    }
    Ok(())
}

fn reject_private_role(
    role_name: &str,
    span: crate::lexer::SourceSpan,
    private: &PrivateNames,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    if private.roles.contains(role_name) {
        return private_error(role_name, span, unit);
    }
    Ok(())
}

fn private_error(
    symbol: &str,
    span: crate::lexer::SourceSpan,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    Err(ModuleError::PrivateDeclarationAccess {
        module: unit.identity().module_path().to_owned(),
        symbol: symbol.to_owned(),
        facade: unit.facade().to_owned(),
        span,
    })
}
