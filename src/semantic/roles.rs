use std::collections::HashSet;

use crate::ast::{PerformDecl, RoleDecl, RoleMethod, TopLevelDecl, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn register_roles(
        &mut self,
        program: &crate::ast::Program,
    ) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Role(role) = declaration else { continue };
            if self.role_types.insert(role.name.clone(), role.clone()).is_some() {
                return Err(role_error(
                    SemanticErrorKind::DuplicateRoleName { name: role.name.clone() },
                    role.span,
                ));
            }
            self.validate_role_methods(role)?;
        }
        Ok(())
    }

    fn validate_role_methods(&mut self, role: &RoleDecl) -> Result<(), SemanticError> {
        let mut names = HashSet::new();
        for method in &role.methods {
            if !names.insert(method.name.clone()) {
                return Err(role_error(
                    SemanticErrorKind::DuplicateRoleMethod {
                        role: role.name.clone(),
                        method: method.name.clone(),
                    },
                    method.span,
                ));
            }
            if method.params.first().is_none_or(|parameter| parameter.name != "self") {
                return Err(role_error(
                    SemanticErrorKind::InvalidRoleReceiver {
                        role: role.name.clone(),
                        method: method.name.clone(),
                    },
                    method.span,
                ));
            }
            for parameter in &method.params {
                self.validate_type_reference(&parameter.ty)?;
            }
            if let Some(return_type) = &method.return_type {
                self.validate_type_reference(return_type)?;
            }
        }
        Ok(())
    }

    pub(super) fn validate_performances(
        &mut self,
        program: &crate::ast::Program,
    ) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Perform(perform) = declaration else { continue };
            self.validate_performance(perform)?;
        }
        Ok(())
    }

    fn validate_performance(&mut self, perform: &PerformDecl) -> Result<(), SemanticError> {
        let Some(role) = self.role_types.get(&perform.role_name).cloned() else {
            return Err(role_error(
                SemanticErrorKind::UnknownRole { name: perform.role_name.clone() },
                perform.span,
            ));
        };
        self.validate_type_reference(&perform.target)?;
        let mut methods = HashSet::new();
        for method in &perform.methods {
            if !methods.insert(method.name.clone()) {
                return Err(role_error(
                    SemanticErrorKind::DuplicateRoleMethod {
                        role: perform.role_name.clone(),
                        method: method.name.clone(),
                    },
                    method.span,
                ));
            }
        }
        for required in &role.methods {
            let Some(implementation) =
                perform.methods.iter().find(|method| method.name == required.name)
            else {
                return Err(role_error(
                    SemanticErrorKind::MissingRoleMethod {
                        role: perform.role_name.clone(),
                        method: required.name.clone(),
                    },
                    perform.span,
                ));
            };
            if !method_matches(required, implementation, &perform.target) {
                return Err(role_error(
                    SemanticErrorKind::RoleMethodMismatch {
                        role: perform.role_name.clone(),
                        method: required.name.clone(),
                    },
                    implementation.span,
                ));
            }
        }
        Ok(())
    }
}

fn method_matches(
    required: &RoleMethod,
    implementation: &crate::ast::VerbDecl,
    target: &TypeName,
) -> bool {
    required.name == implementation.name
        && required.params.len() == implementation.params.len()
        && required.params.iter().zip(&implementation.params).all(|(left, right)| {
            left.role == right.role
                && left.name == right.name
                && type_names_match(&left.ty, &right.ty)
        })
        && required.return_type.as_ref().zip(implementation.return_type.as_ref()).map_or(
            required.return_type.is_none() && implementation.return_type.is_none(),
            |(left, right)| type_names_match(left, right),
        )
        && implementation.params.first().is_some_and(|parameter| {
            parameter.name == "self" && type_names_match(&parameter.ty, target)
        })
}

fn type_names_match(left: &TypeName, right: &TypeName) -> bool {
    left.name == right.name
        && left.arguments.len() == right.arguments.len()
        && left
            .arguments
            .iter()
            .zip(&right.arguments)
            .all(|(left, right)| type_names_match(left, right))
}

fn role_error(kind: SemanticErrorKind, span: SourceSpan) -> SemanticError {
    SemanticError { kind, span }
}
