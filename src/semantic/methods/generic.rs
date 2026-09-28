use crate::ast::{Argument, Expr};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn generic_role_for_expression(&self, expression: &Expr) -> Option<String> {
        let type_name =
            super::super::analyzer::canonical_type_name(&self.resolved_type_name(expression)?);
        self.generic_bounds
            .get(&type_name)
            .and_then(|bounds| bounds.iter().find(|bound| self.role_types.contains_key(*bound)))
            .cloned()
    }

    pub(super) fn visit_generic_bound_method_call(
        &mut self,
        receiver: &Expr,
        role_name: &str,
        method: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let role = self.role_types.get(role_name).cloned().ok_or_else(|| SemanticError {
            kind: SemanticErrorKind::UnknownRole { name: role_name.to_owned() },
            span,
        })?;
        let method_decl =
            role.methods.iter().find(|candidate| candidate.name == method).ok_or_else(|| {
                SemanticError {
                    kind: SemanticErrorKind::UnknownMethod { method: method.to_owned() },
                    span,
                }
            })?;
        let receiver_role =
            method_decl.params.first().map(|parameter| parameter.role.clone()).ok_or_else(
                || SemanticError {
                    kind: SemanticErrorKind::InvalidReceiver { method: method.to_owned() },
                    span,
                },
            )?;
        let signature = super::super::dynamic::role_method_signature(method_decl);
        self.visit_expression(receiver)?;
        if receiver_role == crate::ast::Role::Ins && !self.is_exclusive_owner(receiver) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidReceiver { method: method.to_owned() },
                span,
            });
        }
        let mut parameters = signature.params;
        let mut dispatch = signature.dynamic_params;
        parameters.remove(0);
        dispatch.remove(0);
        let signature = super::super::calls::VerbSignature {
            params: parameters,
            dynamic_params: dispatch,
            ..signature
        };
        self.visit_call_with_signature(method, arguments, span, &signature)?;
        if let Some(return_type) = &signature.return_type_name {
            self.inferred_expression_types.insert((span.start, span.end), return_type.clone());
        }
        Ok(())
    }
}
