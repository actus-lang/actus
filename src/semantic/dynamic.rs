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
            .and_then(|return_type| lookup_builtin_type(&return_type.ty.name)),
        return_type_name: method.return_type.as_ref().map(|return_type| return_type.ty.clone()),
        return_access: method.return_type.as_ref().map(|return_type| return_type.access),
        generic_parameters: Vec::new(),
        external: false,
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
            match declaration {
                TopLevelDecl::Constant(_) => {}
                TopLevelDecl::Verb(verb) => {
                    self.collect_dynamic_parameters(&mut role_names, &verb.params)
                }
                TopLevelDecl::ExternalVerb(verb) => {
                    self.collect_dynamic_parameters(&mut role_names, &verb.params)
                }
                TopLevelDecl::Role(role) => {
                    self.collect_dynamic_methods(&mut role_names, &role.methods)
                }
                TopLevelDecl::Perform(perform) => {
                    for method in &perform.methods {
                        self.collect_dynamic_parameters(&mut role_names, &method.params);
                    }
                }
                TopLevelDecl::Struct(_)
                | TopLevelDecl::Enum(_)
                | TopLevelDecl::Pack(_)
                | TopLevelDecl::Serialize(_)
                | TopLevelDecl::OpenSibling(_)
                | TopLevelDecl::Import(_) => {}
            }
        }
        self.finalize_dynamic_roles(role_names);
    }

    fn collect_dynamic_parameters(&self, role_names: &mut Vec<String>, parameters: &[Param]) {
        for parameter in parameters {
            if parameter.dispatch == DispatchMode::Dynamic
                && !role_names.contains(&parameter.ty.name)
            {
                role_names.push(parameter.ty.name.clone());
            }
        }
    }

    fn collect_dynamic_methods(&self, role_names: &mut Vec<String>, methods: &[RoleMethod]) {
        for method in methods {
            self.collect_dynamic_parameters(role_names, &method.params);
        }
    }

    fn finalize_dynamic_roles(&mut self, mut role_names: Vec<String>) {
        role_names.sort();
        self.model.dynamic_roles = role_names
            .into_iter()
            .map(|role_name| DynamicRoleType { role_name, layout: FatPointerLayout::DYNAMIC_ROLE })
            .collect();
    }
}
