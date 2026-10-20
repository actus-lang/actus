use crate::ast::{Argument, BuiltinType, Expr};
use crate::lexer::SourceSpan;

use super::analyzer::{Analyzer, expression_span};
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(crate) fn validate_print(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<bool, SemanticError> {
        if arguments.len() != 1 {
            return Err(SemanticError {
                kind: SemanticErrorKind::WrongArgumentCount { callee: callee.to_owned() },
                span,
            });
        }
        let argument = &arguments[0];
        let label = argument.name.as_deref();
        if let Some(label) = label
            && !matches!(label, "value" | "string" | "int" | "byte" | "bool" | "buffer")
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownParameter {
                    callee: callee.to_owned(),
                    name: label.to_owned(),
                },
                span: expression_span(&argument.expression),
            });
        }
        self.visit_expression(&argument.expression)?;
        let value_type = self.expression_type(&argument.expression).ok_or_else(|| {
            invalid_print_argument(callee, label.unwrap_or("value"), &argument.expression)
        })?;
        validate_print_type(callee, label, value_type, &argument.expression)
    }
}

fn validate_print_type(
    callee: &str,
    label: Option<&str>,
    value_type: BuiltinType,
    expression: &Expr,
) -> Result<bool, SemanticError> {
    if label == Some("int") && value_type != BuiltinType::Int {
        return Err(invalid_print_argument(callee, "int", expression));
    }
    if label == Some("string") && value_type != BuiltinType::String {
        return Err(invalid_print_argument(callee, "string", expression));
    }
    if label == Some("bool") && value_type != BuiltinType::Bool {
        return Err(invalid_print_argument(callee, "bool", expression));
    }
    if label == Some("buffer") && value_type != BuiltinType::Buffer {
        return Err(invalid_print_argument(callee, "buffer", expression));
    }
    let is_non_scalar_value = matches!(
        value_type,
        BuiltinType::String | BuiltinType::Buffer | BuiltinType::Array | BuiltinType::Map
    );
    if label == Some("byte") && is_non_scalar_value {
        return Err(invalid_print_argument(callee, "byte", expression));
    }
    if !matches!(
        value_type,
        BuiltinType::Int | BuiltinType::String | BuiltinType::Bool | BuiltinType::Buffer
    ) {
        return Err(invalid_print_argument(callee, label.unwrap_or("value"), expression));
    }
    Ok(true)
}

fn invalid_print_argument(callee: &str, parameter: &str, expression: &Expr) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidIntrinsicArgument {
            callee: callee.to_owned(),
            parameter: parameter.to_owned(),
        },
        span: expression_span(expression),
    }
}
