use std::collections::HashSet;

use crate::ast::{Param, PerformDecl, RoleDecl, RoleMethod, TopLevelDecl, TypeName};
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
        }
        Ok(())
    }

    pub(super) fn validate_role_declarations(&mut self) -> Result<(), SemanticError> {
        let roles = self.role_types.values().cloned().collect::<Vec<_>>();
        for role in &roles {
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
            self.validate_role_method(role, method)?;
        }
        Ok(())
    }

    fn validate_role_method(
        &mut self,
        role: &RoleDecl,
        method: &RoleMethod,
    ) -> Result<(), SemanticError> {
        if method.params.first().is_none_or(|parameter| parameter.name != "self") {
            return Err(role_error(
                SemanticErrorKind::InvalidRoleReceiver {
                    role: role.name.clone(),
                    method: method.name.clone(),
                },
                method.span,
            ));
        }
        for (index, parameter) in method.params.iter().enumerate() {
            self.validate_dynamic_parameter(parameter)?;
            self.validate_role_parameter(role, method, index, parameter)?;
        }
        if let Some(return_type) = &method.return_type {
            self.validate_type_reference(&return_type.ty)?;
        }
        Ok(())
    }

    fn validate_role_parameter(
        &mut self,
        role: &RoleDecl,
        method: &RoleMethod,
        index: usize,
        parameter: &Param,
    ) -> Result<(), SemanticError> {
        if parameter.dispatch != crate::ast::DispatchMode::Static {
            return Ok(());
        }
        if parameter.ty.name == "Self" && (index != 0 || !parameter.ty.arguments.is_empty()) {
            return Err(role_error(
                SemanticErrorKind::InvalidRoleReceiver {
                    role: role.name.clone(),
                    method: method.name.clone(),
                },
                parameter.span,
            ));
        }
        if parameter.ty.name != "Self" {
            self.validate_type_reference(&parameter.ty)?;
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
        if perform.role_name == "Drop" {
            return self.validate_drop_performance(perform);
        }
        let Some(role) = self.role_types.get(&perform.role_name).cloned() else {
            return Err(role_error(
                SemanticErrorKind::UnknownRole { name: perform.role_name.clone() },
                perform.span,
            ));
        };
        self.validate_type_reference(&perform.target)?;
        self.validate_performance_methods(&role, perform)?;
        self.register_performance_methods(perform);
        Ok(())
    }

    fn validate_performance_methods(
        &self,
        role: &RoleDecl,
        perform: &PerformDecl,
    ) -> Result<(), SemanticError> {
        let mut methods = HashSet::new();
        self.validate_unique_performance_methods(perform, &mut methods)?;
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

    fn validate_unique_performance_methods(
        &self,
        perform: &PerformDecl,
        names: &mut HashSet<String>,
    ) -> Result<(), SemanticError> {
        for method in &perform.methods {
            if !names.insert(method.name.clone()) {
                return Err(role_error(
                    SemanticErrorKind::DuplicateRoleMethod {
                        role: perform.role_name.clone(),
                        method: method.name.clone(),
                    },
                    method.span,
                ));
            }
        }
        Ok(())
    }

    fn register_performance_methods(&mut self, perform: &PerformDecl) {
        let target_key = canonical_type_name(&perform.target);
        self.performances.insert((perform.role_name.clone(), target_key.clone()));
        for method in &perform.methods {
            self.performance_methods
                .insert((target_key.clone(), method.name.clone()), method.signature());
            self.performance_roles
                .insert((target_key.clone(), method.name.clone()), perform.role_name.clone());
        }
    }

    fn validate_drop_performance(&mut self, perform: &PerformDecl) -> Result<(), SemanticError> {
        let Some(method) = perform.methods.iter().find(|method| method.name == "drop") else {
            return Err(role_error(
                SemanticErrorKind::MissingRoleMethod {
                    role: "Drop".to_owned(),
                    method: "drop".to_owned(),
                },
                perform.span,
            ));
        };
        let valid = method.params.len() == 1
            && method.params[0].role == crate::ast::Role::Ins
            && method.params[0].name == "self"
            && type_names_match(&method.params[0].ty, &perform.target)
            && method.return_type.is_none();
        if !valid {
            return Err(role_error(
                SemanticErrorKind::RoleMethodMismatch {
                    role: "Drop".to_owned(),
                    method: "drop".to_owned(),
                },
                method.span,
            ));
        }
        let target_key = canonical_type_name(&perform.target);
        self.drop_types.insert(target_key.clone());
        self.performance_methods
            .insert((target_key.clone(), "drop".to_owned()), method.signature());
        self.performance_roles.insert((target_key, "drop".to_owned()), "Drop".to_owned());
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
        && required.params.iter().zip(&implementation.params).enumerate().all(
            |(index, (left, right))| {
                left.role == right.role
                    && left.dispatch == right.dispatch
                    && left.name == right.name
                    && (index == 0 || type_names_match(&left.ty, &right.ty))
            },
        )
        && required.return_type.as_ref().zip(implementation.return_type.as_ref()).map_or(
            required.return_type.is_none() && implementation.return_type.is_none(),
            |(left, right)| left.access == right.access && type_names_match(&left.ty, &right.ty),
        )
        && implementation.params.first().is_some_and(|parameter| {
            parameter.name == "self"
                && (is_self_type(&required.params[0].ty) || type_names_match(&parameter.ty, target))
        })
}

fn is_self_type(type_name: &TypeName) -> bool {
    type_name.name == "Self" && type_name.arguments.is_empty()
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

fn canonical_type_name(type_name: &TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

fn role_error(kind: SemanticErrorKind, span: SourceSpan) -> SemanticError {
    SemanticError { kind, span }
}
