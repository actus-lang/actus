use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;
use super::{canonical_type_name, try_type_mismatch};

impl Analyzer {
    pub(crate) fn visit_expression(&mut self, expression: &Expr) -> Result<(), SemanticError> {
        self.record_origin(expression);
        match expression {
            Expr::BufferLiteral { length, .. } => {
                self.visit_expression(length)?;
                self.require_buffer_length(length)
            }
            Expr::Identifier { name, span } => {
                let index = self.binding(name, *span)?;
                self.ensure_readable(index, name, *span)
            }
            Expr::Binary { left, right, .. } => {
                self.visit_expression(left)?;
                self.visit_expression(right)
            }
            Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
                self.visit_expression(expression)
            }
            Expr::Borrow { expression, .. } => self.visit_expression(expression),
            Expr::Try { expression, span } => {
                self.visit_expression(expression)?;
                self.validate_try_expression(expression, *span)
            }
            Expr::Call { callee, arguments, span } => self.visit_call(callee, arguments, *span),
            Expr::MethodCall { receiver, method, arguments, span } => {
                self.visit_method_call(receiver, method, arguments, *span)
            }
            Expr::StructLit { name, type_arguments, fields, span } => {
                self.validate_struct_literal(name, type_arguments, fields, *span)
            }
            Expr::FieldAccess { object, field, span } => {
                if self.enum_receiver_name(object).is_some() {
                    self.validate_enum_unit_variant(object, field, *span)
                } else {
                    self.validate_field_access(object, field, *span)?;
                    self.ensure_field_access_readable(object, field, *span)
                }
            }
            Expr::Case { mode, subject, branches, span } => {
                self.visit_expression(subject)?;
                self.validate_case_patterns(*mode, subject, branches, *span)
            }
            Expr::Integer { .. } | Expr::FloatLiteral { .. } | Expr::StringLiteral { .. } => Ok(()),
        }
    }

    fn validate_try_expression(
        &self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expected) = self.current_return_type_name.as_ref() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    callee: "?".to_owned(),
                    parameter: "return type".to_owned(),
                    expected: "Result[T, E]".to_owned(),
                    found: "no return type".to_owned(),
                },
                span,
            });
        };
        let Some(operand) = self.enum_type_application(expression) else {
            return Err(try_type_mismatch(span, expected, "unknown"));
        };
        if expected.name != "Result"
            || expected.arguments.len() != 2
            || operand.name != "Result"
            || operand.arguments.len() != 2
            || canonical_type_name(&expected.arguments[1])
                != canonical_type_name(&operand.arguments[1])
        {
            return Err(try_type_mismatch(span, expected, &canonical_type_name(&operand)));
        }
        Ok(())
    }
}
