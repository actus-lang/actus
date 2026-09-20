use crate::ast::{Argument, Expr, Role, VerbDecl};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::BindingState;

#[derive(Clone)]
pub(super) struct VerbSignature {
    pub(super) params: Vec<(String, Role, String)>,
}

impl VerbDecl {
    pub(super) fn signature(&self) -> VerbSignature {
        VerbSignature {
            params: self
                .params
                .iter()
                .map(|parameter| {
                    (parameter.name.clone(), parameter.role.clone(), parameter.ty.name.clone())
                })
                .collect(),
        }
    }
}

impl Analyzer {
    pub(super) fn visit_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(signature) = self.signatures.get(callee).cloned() else {
            for argument in arguments {
                self.visit_expression(&argument.expression)?;
            }
            return Ok(());
        };
        let parameter_indices = self.bind_arguments(callee, &signature, arguments, span)?;
        for argument in arguments {
            self.visit_expression(&argument.expression)?;
        }
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            let (_, role, _) = &signature.params[parameter_index];
            self.validate_argument_role(
                callee,
                &signature.params[parameter_index].0,
                role,
                &argument.expression,
            )?;
            if *role == Role::Dat {
                self.move_dat_argument(&argument.expression, argument_span(argument))?;
            }
        }
        Ok(())
    }

    fn bind_arguments(
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

    fn validate_argument_role(
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

    fn is_owner_argument(&self, expression: &Expr) -> bool {
        let Expr::Identifier { name, span } = expression else { return false };
        let Ok(index) = self.binding(name, *span) else { return false };
        matches!(self.model.bindings[index].role, Role::Erg | Role::Dat)
            && matches!(self.model.bindings[index].state, BindingState::Active)
    }

    fn is_borrow_argument(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Identifier { name, span } => {
                let Ok(index) = self.binding(name, *span) else { return false };
                self.model.bindings[index].role == Role::Abs
                    && !matches!(
                        self.model.bindings[index].state,
                        BindingState::Moved | BindingState::Dropped
                    )
            }
            Expr::Borrow { expression, .. } => self.is_owner_argument(expression),
            _ => false,
        }
    }

    fn move_dat_argument(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Expr::Identifier { name, span: identifier_span } = expression else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: "expression".to_owned() },
                span,
            });
        };
        let index = self.binding(name, *identifier_span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: name.clone() },
                span,
            });
        }
        match self.model.bindings[index].state {
            BindingState::Active => self.model.bindings[index].state = BindingState::Moved,
            BindingState::Frozen { .. } => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MoveFrozen {
                        name: name.clone(),
                        borrow_ids: self.blocking_borrow_ids(index),
                    },
                    span,
                });
            }
            BindingState::Moved | BindingState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                    span,
                });
            }
        }
        Ok(())
    }
}

fn argument_span(argument: &Argument) -> SourceSpan {
    expression_span(&argument.expression)
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. } => *span,
    }
}
