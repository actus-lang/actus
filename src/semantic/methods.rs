use crate::ast::{Argument, BuiltinType, Expr, Program, Role, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn validate_method_declarations(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration else { continue };
            let Some(receiver) = verb.params.first() else { continue };
            if receiver.name != "self" {
                continue;
            }
            if !matches!(receiver.role, Role::Erg | Role::Abs)
                || !self.struct_types.contains_key(&receiver.ty.name)
            {
                return Err(SemanticError {
                    kind: SemanticErrorKind::InvalidReceiver { method: verb.name.clone() },
                    span: receiver.span,
                });
            }
        }
        Ok(())
    }

    pub(super) fn visit_method_call(
        &mut self,
        receiver: &Expr,
        method: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Some(role_name) = self.dynamic_role_for_expression(receiver) {
            return self.visit_dynamic_method_call(receiver, &role_name, method, arguments, span);
        }
        if self.enum_receiver_name(receiver).is_some() {
            return self.validate_enum_constructor(receiver, method, arguments, span);
        }
        if method == "raw_slice" {
            return self.visit_raw_slice_call(receiver, arguments, span);
        }
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
        if &actual_type != receiver_type {
            return Err(SemanticError {
                kind: SemanticErrorKind::ReceiverTypeMismatch {
                    method: method.to_owned(),
                    expected: receiver_type.clone(),
                    found: actual_type,
                },
                span,
            });
        }
        let combined = self.method_arguments(receiver, receiver_role, receiver_name, arguments);
        if performance.is_some() {
            if let Some(role_name) = self.performance_role(&actual_type, method).map(str::to_owned)
            {
                self.mark_reachable_performance(&actual_type, method, &role_name);
            }
            self.visit_call_with_signature(method, &combined, span, &signature)
        } else {
            self.visit_call(method, &combined, span)
        }
    }

    fn visit_raw_slice_call(
        &mut self,
        receiver: &Expr,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(receiver)?;
        if self.expression_type(receiver) != Some(BuiltinType::Buffer) || arguments.len() != 2 {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: "raw_slice".to_owned(),
                    parameter: "receiver and bounds".to_owned(),
                },
                span,
            });
        }
        for argument in arguments {
            self.visit_expression(&argument.expression)?;
            if self.expression_type(&argument.expression) != Some(BuiltinType::Int) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::InvalidIntrinsicArgument {
                        callee: "raw_slice".to_owned(),
                        parameter: "start and length".to_owned(),
                    },
                    span: expression_span(&argument.expression),
                });
            }
        }
        Ok(())
    }

    fn visit_dynamic_method_call(
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
        let signature = super::dynamic::role_method_signature(&method);
        let combined = self.method_arguments(receiver, &Role::Abs, "self", arguments);
        self.mark_dynamic_role_performances(role_name, method.name.as_str());
        self.visit_call_with_signature(method.name.as_str(), &combined, span, &signature)
    }

    fn method_signature(
        &self,
        method: &str,
        span: SourceSpan,
    ) -> Result<super::calls::VerbSignature, SemanticError> {
        let Some(signature) = self.signatures.get(method).cloned() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownMethod { method: method.to_owned() },
                span,
            });
        };
        let valid = signature
            .params
            .first()
            .is_some_and(|(name, role, _)| name == "self" && matches!(role, Role::Erg | Role::Abs));
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
        let receiver_expression = match role {
            Role::Erg => receiver.clone(),
            Role::Abs if self.is_abs_binding(receiver) => receiver.clone(),
            Role::Abs => Expr::Borrow {
                expression: Box::new(receiver.clone()),
                span: expression_span(receiver),
            },
            Role::Dat | Role::Ins => receiver.clone(),
        };
        let named = arguments.iter().any(|argument| argument.name.is_some());
        let mut combined = Vec::with_capacity(arguments.len() + 1);
        combined.push(Argument {
            name: named.then(|| receiver_name.to_owned()),
            role: None,
            role_span: None,
            expression: receiver_expression,
        });
        combined.extend(arguments.iter().cloned());
        combined
    }

    fn is_abs_binding(&self, expression: &Expr) -> bool {
        let Expr::Identifier { name, span } = expression else { return false };
        self.binding(name, *span)
            .ok()
            .is_some_and(|index| self.model.bindings[index].role == Role::Abs)
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Case { span, .. } => *span,
    }
}
