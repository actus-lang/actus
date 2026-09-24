use crate::ast::{
    DispatchMode, Param, Program, Role, RoleMethod, TopLevelDecl, lookup_builtin_type,
};

use super::analyzer::Analyzer;
use super::calls::VerbSignature;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::{DynamicRoleType, FatPointerLayout};

impl Analyzer {
    pub(super) fn mark_dynamic_role_performances(&mut self, role: &str, method: &str) {
        let targets = self
            .performances
            .iter()
            .filter(|(candidate_role, _)| candidate_role == role)
            .map(|(_, target)| target.clone())
            .collect::<Vec<_>>();
        for target in targets {
            self.mark_reachable_performance(&target, method, role);
        }
    }

    pub(super) fn dynamic_role_for_expression(
        &self,
        expression: &crate::ast::Expr,
    ) -> Option<String> {
        let crate::ast::Expr::Identifier { name, span } = expression else { return None };
        let index = self.binding(name, *span).ok()?;
        self.binding_dynamic_roles.get(&index).cloned()
    }
}

pub(super) fn role_method_signature(method: &RoleMethod) -> VerbSignature {
    VerbSignature {
        params: method
            .params
            .iter()
            .map(|parameter| {
                (parameter.name.clone(), parameter.role.clone(), parameter.ty.name.clone())
            })
            .collect(),
        dynamic_params: method.params.iter().map(|parameter| parameter.dispatch).collect(),
        return_type: method
            .return_type
            .as_ref()
            .and_then(|type_name| lookup_builtin_type(&type_name.name)),
    }
}

impl Analyzer {
    pub(super) fn validate_dynamic_parameter(
        &self,
        parameter: &Param,
    ) -> Result<(), SemanticError> {
        if parameter.dispatch == DispatchMode::Static {
            return Ok(());
        }
        if parameter.role != Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::DynamicRequiresAbs { parameter: parameter.name.clone() },
                span: parameter.span,
            });
        }
        if !self.role_types.contains_key(&parameter.ty.name) {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownDynamicRole { name: parameter.ty.name.clone() },
                span: parameter.ty.span,
            });
        }
        Ok(())
    }

    pub(super) fn collect_dynamic_roles(&mut self, program: &Program) {
        let mut role_names = Vec::new();
        for declaration in &program.declarations {
            let parameters = match declaration {
                TopLevelDecl::Verb(verb) => &verb.params,
                TopLevelDecl::ExternalVerb(verb) => &verb.params,
                TopLevelDecl::Role(role) => {
                    for method in &role.methods {
                        for parameter in &method.params {
                            if parameter.dispatch == DispatchMode::Dynamic
                                && !role_names.contains(&parameter.ty.name)
                            {
                                role_names.push(parameter.ty.name.clone());
                            }
                        }
                    }
                    continue;
                }
                TopLevelDecl::Perform(perform) => {
                    for method in &perform.methods {
                        for parameter in &method.params {
                            if parameter.dispatch == DispatchMode::Dynamic
                                && !role_names.contains(&parameter.ty.name)
                            {
                                role_names.push(parameter.ty.name.clone());
                            }
                        }
                    }
                    continue;
                }
                TopLevelDecl::Struct(_) | TopLevelDecl::Enum(_) => continue,
            };
            for parameter in parameters {
                if parameter.dispatch == DispatchMode::Dynamic
                    && !role_names.contains(&parameter.ty.name)
                {
                    role_names.push(parameter.ty.name.clone());
                }
            }
        }
        role_names.sort();
        self.model.dynamic_roles = role_names
            .into_iter()
            .map(|role_name| DynamicRoleType { role_name, layout: FatPointerLayout::DYNAMIC_ROLE })
            .collect();
    }
}
