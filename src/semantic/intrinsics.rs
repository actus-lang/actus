use std::collections::HashSet;

use crate::ast::{Argument, Expr, IntrinsicKind, lookup_call_intrinsic, lookup_intrinsic};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn is_reserved_name(name: &str) -> bool {
    name == "allocate" || lookup_intrinsic(name).is_some()
}

impl Analyzer {
    pub(super) fn visit_intrinsic_call(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let Some(kind) = lookup_call_intrinsic(callee) else { return Ok(false) };
        match kind {
            IntrinsicKind::Append => {
                let arguments = self.bind_intrinsic_arguments(callee, arguments, span)?;
                for argument in &arguments {
                    self.visit_expression(&argument.expression)?;
                }
                let handle = &arguments[0].expression;
                let parameters = IntrinsicKind::Append.spec().parameters;
                if !self.is_owner_argument(handle)
                    || self.expression_type(handle) != Some(crate::ast::BuiltinType::Buffer)
                {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::InvalidArgumentRole {
                            callee: callee.to_owned(),
                            parameter: parameters[0].to_owned(),
                        },
                        span: expression_span(handle),
                    });
                }
                self.require_intrinsic_type(
                    callee,
                    parameters[1],
                    crate::ast::BuiltinType::Int,
                    &arguments[1].expression,
                )?;
                Ok(true)
            }
            IntrinsicKind::Print => self.validate_print(callee, arguments, span),
            IntrinsicKind::Drop => unreachable!("call lookup excludes statement intrinsics"),
        }
    }

    fn validate_print(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let arguments = self.bind_intrinsic_arguments(callee, arguments, span)?;
        let argument = arguments[0];
        self.visit_expression(&argument.expression)?;
        let value_type =
            self.expression_type(&argument.expression).ok_or_else(|| SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: callee.to_owned(),
                    parameter: IntrinsicKind::Print.spec().parameters[0].to_owned(),
                },
                span: expression_span(&argument.expression),
            })?;
        if !matches!(value_type, crate::ast::BuiltinType::Int | crate::ast::BuiltinType::String) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: callee.to_owned(),
                    parameter: IntrinsicKind::Print.spec().parameters[0].to_owned(),
                },
                span: expression_span(&argument.expression),
            });
        }
        Ok(true)
    }

    fn bind_intrinsic_arguments<'a>(
        &self,
        callee: &str,
        arguments: &'a [Argument],
        span: SourceSpan,
    ) -> Result<Vec<&'a Argument>, SemanticError> {
        let parameter_names = lookup_call_intrinsic(callee)
            .expect("intrinsic call should resolve before argument binding")
            .spec()
            .parameters;
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

    fn require_intrinsic_type(
        &self,
        callee: &str,
        parameter: &str,
        expected: crate::ast::BuiltinType,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        if self.expression_type(expression) == Some(expected) {
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
