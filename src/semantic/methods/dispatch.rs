use crate::ast::{Argument, Expr, Role};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(crate) fn visit_method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Some(result) = self.visit_special_method_call(receiver, method, arguments, span) {
            return result;
        }
        self.visit_static_method_call(receiver, method, arguments, span)
    }

    fn visit_special_method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Option<Result<(), SemanticError>> {
        if let Some(role_name) = self.dynamic_role_for_expression(receiver) {
            return Some(
                self.visit_dynamic_method_call(receiver, &role_name, method, arguments, span),
            );
        }
        if let Some(role_name) = self.generic_role_for_expression(receiver) {
            return Some(
                self.visit_generic_bound_method_call(receiver, &role_name, method, arguments, span),
            );
        }
        if let Some(result) = self.try_visit_arena_place(receiver, method, arguments, span) {
            return Some(result);
        }
        if self.enum_receiver_name(receiver).is_some() {
            return Some(self.validate_enum_constructor(receiver, method, arguments, span));
        }
        (method == "raw_slice").then(|| self.visit_raw_slice_call(receiver, arguments, span))
    }

    fn visit_static_method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(receiver)?;
        let Some(actual_type) = self.expression_struct_type(receiver) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidReceiver { method: method.to_owned() },
                span,
            });
        };
        let performance = self.performance_signature(&actual_type, method);
        let signature =
            performance.clone().map(Ok).unwrap_or_else(|| self.method_signature(method, span))?;
        let (receiver_name, receiver_role, receiver_type) =
            signature.params.first().ok_or_else(|| SemanticError {
                kind: SemanticErrorKind::InvalidReceiver { method: method.to_owned() },
                span,
            })?;
        self.validate_receiver_type(&actual_type, receiver_type, method, span)?;
        let combined = self.method_arguments(receiver, receiver_role, receiver_name, arguments);
        if performance.is_some() {
            self.dispatch_performance_method(&actual_type, method, &combined, span, &signature)
        } else {
            self.visit_call(method, &combined, span)
        }
    }

    fn validate_receiver_type(
        &self,
        actual_type: &str,
        receiver_type: &str,
        method: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if actual_type == receiver_type {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::ReceiverTypeMismatch {
                method: method.to_owned(),
                expected: receiver_type.to_owned(),
                found: actual_type.to_owned(),
            },
            span,
        })
    }

    fn dispatch_performance_method(
        &mut self,
        actual_type: &str,
        method: &str,
        arguments: &[Argument],
        span: SourceSpan,
        signature: &super::super::calls::VerbSignature,
    ) -> Result<(), SemanticError> {
        if let Some(role_name) = self.performance_role(actual_type, method).map(str::to_owned) {
            self.mark_reachable_performance(actual_type, method, &role_name);
        }
        self.visit_call_with_signature(method, arguments, span, signature)
    }

    fn method_signature(
        &self,
        method: &str,
        span: SourceSpan,
    ) -> Result<super::super::calls::VerbSignature, SemanticError> {
        let Some(signature) = self.signatures.get(method).cloned() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownMethod { method: method.to_owned() },
                span,
            });
        };
        let valid = signature.params.first().is_some_and(|(name, role, _)| {
            name == "self" && matches!(role, Role::Erg | Role::Abs | Role::Dat | Role::Ins)
        });
        if valid {
            Ok(signature)
        } else {
            Err(SemanticError {
                kind: SemanticErrorKind::InvalidReceiver { method: method.to_owned() },
                span,
            })
        }
    }

    fn method_arguments(
        &self,
        receiver: &Expr,
        role: &Role,
        receiver_name: &str,
        arguments: &[Argument],
    ) -> Vec<Argument> {
        let named = arguments.iter().any(|argument| argument.name.is_some());
        let mut combined = Vec::with_capacity(arguments.len() + 1);
        combined.push(Argument {
            name: named.then(|| receiver_name.to_owned()),
            role: matches!(role, Role::Ins).then_some(Role::Ins),
            role_span: None,
            expression: receiver.clone(),
        });
        combined.extend(arguments.iter().cloned());
        combined
    }
}

pub(super) fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::Cast { span, .. }
        | Expr::If { span, .. } => *span,
    }
}
