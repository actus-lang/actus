use std::collections::HashSet;

use crate::ast::{
    Argument, Expr, IntrinsicKind, PrimitiveType, Role, TypeName, lookup_call_intrinsic,
    lookup_intrinsic, primitive_type,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn is_reserved_name(name: &str) -> bool {
    // `copy` is also a public standard-library verb name. Intrinsic dispatch
    // already yields to a locally registered signature, so the name must remain
    // available to module-scoped declarations without weakening the intrinsic
    // fallback for scalar reuse.
    name == "allocate" || (lookup_intrinsic(name).is_some() && !matches!(name, "print" | "copy"))
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
                self.require_append_byte_type(callee, parameters[1], &arguments[1].expression)?;
                Ok(true)
            }
            IntrinsicKind::Crc32 => self.validate_crc32(arguments, callee, span),
            IntrinsicKind::Crc32Matches => self.validate_crc32_matches(arguments, callee, span),
            IntrinsicKind::ValidateFixedFrame => self.validate_fixed_frame(arguments, callee, span),
            IntrinsicKind::Copy => self.validate_copy(arguments, callee, span),
            IntrinsicKind::Print => self.validate_print(callee, arguments, span),
            IntrinsicKind::Drop => unreachable!("call lookup excludes statement intrinsics"),
        }
    }

    fn validate_crc32(
        &mut self,
        arguments: &[Argument],
        callee: &str,
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let arguments = self.bind_intrinsic_arguments(callee, arguments, span)?;
        for argument in &arguments {
            self.visit_expression(&argument.expression)?;
        }
        let buffer = &arguments[0].expression;
        if self.expression_type(buffer) != Some(crate::ast::BuiltinType::Buffer)
            || !self.is_readable_owner(buffer)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: callee.to_owned(),
                    parameter: "buffer".to_owned(),
                },
                span: expression_span(buffer),
            });
        }
        for argument in &arguments[1..] {
            if !self.is_integer_expression(&argument.expression) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::InvalidIntrinsicArgument {
                        callee: callee.to_owned(),
                        parameter: "range".to_owned(),
                    },
                    span: expression_span(&argument.expression),
                });
            }
        }
        Ok(true)
    }

    fn validate_crc32_matches(
        &mut self,
        arguments: &[Argument],
        callee: &str,
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let arguments = self.bind_intrinsic_arguments(callee, arguments, span)?;
        for argument in &arguments {
            self.visit_expression(&argument.expression)?;
        }
        let buffer = &arguments[0].expression;
        if self.expression_type(buffer) != Some(crate::ast::BuiltinType::Buffer)
            || !self.is_readable_owner(buffer)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: callee.to_owned(),
                    parameter: "buffer".to_owned(),
                },
                span: expression_span(buffer),
            });
        }
        if arguments[1..].iter().any(|argument| !self.is_integer_expression(&argument.expression)) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: callee.to_owned(),
                    parameter: "range".to_owned(),
                },
                span,
            });
        }
        Ok(true)
    }

    fn validate_fixed_frame(
        &mut self,
        arguments: &[Argument],
        callee: &str,
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let arguments = self.bind_intrinsic_arguments(callee, arguments, span)?;
        for argument in &arguments {
            self.visit_expression(&argument.expression)?;
        }
        let buffer = &arguments[0].expression;
        if self.expression_type(buffer) != Some(crate::ast::BuiltinType::Buffer)
            || !self.is_readable_owner(buffer)
            || arguments[1..]
                .iter()
                .any(|argument| !self.is_integer_expression(&argument.expression))
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: callee.to_owned(),
                    parameter: "frame".to_owned(),
                },
                span,
            });
        }
        Ok(true)
    }

    fn is_integer_expression(&self, expression: &Expr) -> bool {
        if matches!(expression, Expr::Integer { .. }) {
            return true;
        }
        self.expression_type_name(expression).is_some_and(|name| {
            primitive_type(&name)
                .is_some_and(|primitive| matches!(primitive, PrimitiveType::Integer { .. }))
        })
    }

    fn validate_copy(
        &mut self,
        arguments: &[Argument],
        callee: &str,
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        let arguments = self.bind_intrinsic_arguments(callee, arguments, span)?;
        let argument = arguments[0];
        self.visit_expression(&argument.expression)?;
        if argument.role != Some(Role::Abs) || !self.is_readable_owner(&argument.expression) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidArgumentRole {
                    callee: callee.to_owned(),
                    parameter: IntrinsicKind::Copy.spec().parameters[0].to_owned(),
                },
                span: expression_span(&argument.expression),
            });
        }
        let type_name = self.expression_type_name(&argument.expression);
        let eligible = type_name.as_deref().is_some_and(|name| {
            name == "Int"
                || name == "Bool"
                || primitive_type(name)
                    .is_some_and(|primitive| matches!(primitive, PrimitiveType::Integer { .. }))
        });
        if !eligible {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidIntrinsicArgument {
                    callee: callee.to_owned(),
                    parameter: IntrinsicKind::Copy.spec().parameters[0].to_owned(),
                },
                span: expression_span(&argument.expression),
            });
        }
        if let Some(name) = type_name {
            self.inferred_expression_types.insert(
                (span.start, span.end),
                TypeName { name, arguments: Vec::new(), reference_role: None, span },
            );
        }
        Ok(true)
    }

    fn require_append_byte_type(
        &self,
        callee: &str,
        parameter: &str,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let is_integer = self.expression_type_name(expression).is_some_and(|name| {
            name == "Int"
                || primitive_type(&name)
                    .is_some_and(|primitive| matches!(primitive, PrimitiveType::Integer { .. }))
        });
        if is_integer {
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
        self.validate_named_intrinsic_arguments(callee, arguments, parameter_names, span)?;
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

    fn validate_named_intrinsic_arguments(
        &self,
        callee: &str,
        arguments: &[Argument],
        parameter_names: &[&str],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Some(name) = arguments.iter().find_map(|argument| {
            argument.name.as_deref().filter(|name| !parameter_names.contains(name))
        }) {
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
        if unique_names.len() == arguments.len() {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::DuplicateArgument { name: "intrinsic parameter".to_owned() },
            span,
        })
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => *span,
    }
}
