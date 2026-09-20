use crate::ast::{Argument, Expr, Role, VerbDecl};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::BindingState;

#[derive(Clone)]
pub(super) struct VerbSignature {
    pub(super) params: Vec<(String, Role)>,
}

impl VerbDecl {
    pub(super) fn signature(&self) -> VerbSignature {
        VerbSignature {
            params: self
                .params
                .iter()
                .map(|parameter| (parameter.name.clone(), parameter.role.clone()))
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
            if signature.params[parameter_index].1 == Role::Dat {
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
                    kind: SemanticErrorKind::MoveFrozen { name: name.clone() },
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
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. } => *span,
    }
}
