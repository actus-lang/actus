use crate::ast::{Argument, DispatchMode, Expr, Role, lookup_builtin_type};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::calls::VerbSignature;
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::OwnershipState;

impl Analyzer {
    pub(super) fn validate_argument_type(
        &self,
        callee: &str,
        parameter: &str,
        expected: &str,
        dispatch: DispatchMode,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        if dispatch == DispatchMode::Dynamic {
            let Some(found) = self.expression_type_name(expression) else { return Ok(()) };
            if self.has_performance(
                expected,
                &crate::ast::TypeName {
                    name: found.clone(),
                    arguments: Vec::new(),
                    span: expression_span(expression),
                },
            ) {
                return Ok(());
            }
            return Err(SemanticError {
                kind: SemanticErrorKind::DynamicRoleMismatch { role: expected.to_owned(), found },
                span: expression_span(expression),
            });
        }
        let Some(found) = self.expression_type(expression) else { return Ok(()) };
        let Some(expected_type) = lookup_builtin_type(expected) else { return Ok(()) };
        if found == expected_type {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::TypeMismatch {
                callee: callee.to_owned(),
                parameter: parameter.to_owned(),
                expected: expected.to_owned(),
                found: found.spec().name.to_owned(),
            },
            span: expression_span(expression),
        })
    }

    pub(super) fn bind_arguments(
        &self,
        callee: &str,
        signature: &VerbSignature,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<Vec<usize>, SemanticError> {
        let named = arguments.iter().any(|argument| argument.name.is_some());
        let positional = arguments.iter().any(|argument| argument.name.is_none());
        if named && positional {
            return Err(SemanticError {
                kind: SemanticErrorKind::MixedArgumentModes { callee: callee.to_owned() },
                span,
            });
        }
        if arguments.len() != signature.params.len() {
            return Err(SemanticError {
                kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                span,
            });
        }
        if !named {
            self.validate_positional_call(callee, signature, span)?;
            return Ok((0..arguments.len()).collect());
        }
        let mut indices = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let name = argument.name.as_ref().expect("named call argument");
            let Some(index) = signature.params.iter().position(|parameter| parameter.0 == *name)
            else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UnknownParameter {
                        callee: callee.to_owned(),
                        name: name.clone(),
                    },
                    span: argument_span(argument),
                });
            };
            if indices.contains(&index) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateArgument { name: name.clone() },
                    span: argument_span(argument),
                });
            }
            indices.push(index);
        }
        Ok(indices)
    }

    fn validate_positional_call(
        &self,
        callee: &str,
        signature: &VerbSignature,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        for left in 0..signature.params.len() {
            for right in (left + 1)..signature.params.len() {
                let (_, left_role, left_type) = &signature.params[left];
                let (_, right_role, right_type) = &signature.params[right];
                if left_role == right_role && left_type == right_type {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::AmbiguousPositionalCall {
                            callee: callee.to_owned(),
                        },
                        span,
                    });
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate_argument_role(
        &self,
        callee: &str,
        parameter: &str,
        role: &Role,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let valid = match role {
            Role::Dat => true,
            Role::Erg => self.is_owner_argument(expression),
            Role::Abs => self.is_borrow_argument(expression),
            Role::Ins => false,
        };
        if valid {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidArgumentRole {
                callee: callee.to_owned(),
                parameter: parameter.to_owned(),
            },
            span: expression_span(expression),
        })
    }

    pub(super) fn is_owner_argument(&self, expression: &Expr) -> bool {
        let Expr::Identifier { name, span } = expression else { return false };
        let Ok(index) = self.binding(name, *span) else { return false };
        matches!(self.model.bindings[index].role, Role::Erg | Role::Dat)
            && matches!(self.model.bindings[index].ownership, OwnershipState::Active)
    }

    fn is_borrow_argument(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Identifier { name, span } => {
                let Ok(index) = self.binding(name, *span) else { return false };
                self.model.bindings[index].role == Role::Abs
                    && !matches!(
                        self.model.bindings[index].ownership,
                        OwnershipState::PartiallyMoved { .. }
                            | OwnershipState::Moved
                            | OwnershipState::Dropped
                    )
            }
            Expr::Borrow { expression, .. } => self.is_readable_owner(expression),
            _ => false,
        }
    }

    fn is_readable_owner(&self, expression: &Expr) -> bool {
        let Expr::Identifier { name, span } = expression else { return false };
        let Ok(index) = self.binding(name, *span) else { return false };
        matches!(self.model.bindings[index].role, Role::Erg | Role::Dat)
            && matches!(
                self.model.bindings[index].ownership,
                OwnershipState::Active | OwnershipState::PartiallyMoved { .. }
            )
    }
}

pub(super) fn argument_span(argument: &Argument) -> SourceSpan {
    expression_span(&argument.expression)
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
