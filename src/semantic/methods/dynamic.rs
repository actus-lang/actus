use crate::ast::{Argument, Expr, Role};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn visit_dynamic_method_call(
        &mut self,
        receiver: &Expr,
        role_name: &str,
        method_name: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let role = self.role_types.get(role_name).cloned().ok_or_else(|| SemanticError {
            kind: SemanticErrorKind::UnknownRole { name: role_name.to_owned() },
            span,
        })?;
        let method = role
            .methods
            .iter()
            .find(|candidate| candidate.name == method_name)
            .cloned()
            .ok_or_else(|| SemanticError {
            kind: SemanticErrorKind::UnknownMethod { method: method_name.to_owned() },
            span,
        })?;
        let Some(receiver_parameter) = method.params.first() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidReceiver { method: method_name.to_owned() },
                span,
            });
        };
        if receiver_parameter.role != Role::Abs
            || self
                .root_binding_index(receiver)
                .is_none_or(|index| self.model.bindings[index].role != Role::Abs)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidReceiver { method: method_name.to_owned() },
                span,
            });
        }
        self.visit_expression(receiver)?;
        let signature = super::super::dynamic::role_method_signature(&method);
        let mut parameters = signature.params;
        let mut dispatch = signature.dynamic_params;
        parameters.remove(0);
        dispatch.remove(0);
        let signature = super::super::calls::VerbSignature {
            params: parameters,
            dynamic_params: dispatch,
            ..signature
        };
        self.mark_dynamic_role_performances(role_name, method.name.as_str());
        self.visit_call_with_signature(method.name.as_str(), arguments, span, &signature)
    }
}
