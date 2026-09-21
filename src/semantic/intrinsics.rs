use std::collections::HashSet;

use crate::ast::{Argument, Expr};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn is_reserved_name(name: &str) -> bool {
    matches!(name, "allocate" | "append")
}

impl Analyzer {
    pub(super) fn visit_intrinsic_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        match callee {
            "allocate" => {
                let arguments =
                    self.bind_intrinsic_arguments(callee, arguments, &["length"], span)?;
                let argument = arguments[0];
                self.visit_expression(&argument.expression)?;
                self.require_scalar_argument(callee, "length", &argument.expression)?;
                Ok(true)
            }
            "append" => {
                let arguments =
                    self.bind_intrinsic_arguments(callee, arguments, &["handle", "byte"], span)?;
                for argument in &arguments {
                    self.visit_expression(&argument.expression)?;
                }
                let handle = &arguments[0].expression;
                if !self.is_owner_argument(handle) {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::InvalidArgumentRole {
                            callee: callee.to_owned(),
                            parameter: "handle".to_owned(),
                        },
                        span: expression_span(handle),
                    });
                }
                self.require_scalar_argument(callee, "byte", &arguments[1].expression)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn bind_intrinsic_arguments<'a>(
        &self,
        callee: &str,
        arguments: &'a [Argument],
        parameter_names: &[&str],
        span: SourceSpan,
    ) -> Result<Vec<&'a Argument>, SemanticError> {
        self.validate_intrinsic_shape(callee, arguments, parameter_names, span)?;
        let named = arguments.iter().any(|argument| argument.name.is_some());
        if !named {
            return Ok(arguments.iter().collect());
        }
        self.bind_named_intrinsic_arguments(callee, arguments, parameter_names, span)
    }

    fn validate_intrinsic_shape(
        &self,
        callee: &str,
        arguments: &[Argument],
        parameter_names: &[&str],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if arguments.len() != parameter_names.len() {
            return Err(SemanticError {
                kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                span,
            });
        }
        let named = arguments.iter().any(|argument| argument.name.is_some());
        let positional = arguments.iter().any(|argument| argument.name.is_none());
        if named && positional {
            return Err(SemanticError {
                kind: SemanticErrorKind::MixedArgumentModes { callee: callee.to_owned() },
                span,
            });
        }
        Ok(())
    }

    fn bind_named_intrinsic_arguments<'a>(
        &self,
        callee: &str,
        arguments: &'a [Argument],
        parameter_names: &[&str],
        span: SourceSpan,
    ) -> Result<Vec<&'a Argument>, SemanticError> {
        let unknown_name = arguments.iter().find_map(|argument| {
            argument.name.as_deref().filter(|name| !parameter_names.contains(name))
        });
        if let Some(name) = unknown_name {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownParameter {
                    callee: callee.to_owned(),
                    name: name.to_owned(),
                },
                span,
            });
        }
        let unique_names = arguments
            .iter()
            .filter_map(|argument| argument.name.as_deref())
            .collect::<HashSet<_>>();
        if unique_names.len() != arguments.len() {
            return Err(SemanticError {
                kind: SemanticErrorKind::DuplicateArgument {
                    name: "intrinsic parameter".to_owned(),
                },
                span,
            });
        }
        let mut ordered = Vec::with_capacity(parameter_names.len());
        for parameter_name in parameter_names {
            let Some(argument) =
                arguments.iter().find(|argument| argument.name.as_deref() == Some(*parameter_name))
            else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UnknownParameter {
                        callee: callee.to_owned(),
                        name: parameter_name.to_string(),
                    },
                    span,
                });
            };
            ordered.push(argument);
        }
        Ok(ordered)
    }

    fn require_scalar_argument(
        &self,
        callee: &str,
        parameter: &str,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        if is_scalar_expression(expression) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidIntrinsicArgument {
                callee: callee.to_owned(),
                parameter: parameter.to_owned(),
            },
            span: expression_span(expression),
        })
    }
}

fn is_scalar_expression(expression: &Expr) -> bool {
    match expression {
        Expr::StringLiteral { .. } | Expr::Borrow { .. } => false,
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            is_scalar_expression(expression)
        }
        Expr::Binary { left, right, .. } => {
            is_scalar_expression(left) && is_scalar_expression(right)
        }
        Expr::Identifier { .. } | Expr::Integer { .. } | Expr::Call { .. } => true,
    }
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
