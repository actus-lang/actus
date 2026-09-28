use crate::ast::{Expr, PrimitiveType, TypeName, primitive_type};

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(crate) fn validate_expected_literal(
        &self,
        expression: &Expr,
        expected: &TypeName,
    ) -> Result<(), SemanticError> {
        let Some(PrimitiveType::Integer { signed, width }) = primitive_type(&expected.name) else {
            return Ok(());
        };
        let Some((literal, negative)) = integer_literal(expression) else { return Ok(()) };
        let magnitude = parse_integer_magnitude(&literal).ok_or_else(|| SemanticError {
            kind: SemanticErrorKind::NumericLiteralOutOfRange {
                ty: expected.name.clone(),
                literal: format_literal(&literal, negative),
            },
            span: expected.span,
        })?;
        let valid = if signed {
            signed_literal_fits(magnitude, negative, width)
        } else {
            !negative && magnitude <= unsigned_maximum(width)
        };
        if valid {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::NumericLiteralOutOfRange {
                ty: expected.name.clone(),
                literal: format_literal(&literal, negative),
            },
            span: expected.span,
        })
    }
}

fn integer_literal(expression: &Expr) -> Option<(String, bool)> {
    match expression {
        Expr::Integer { value, .. } => Some((value.clone(), false)),
        Expr::Grouping { expression, .. } => integer_literal(expression),
        Expr::Unary { operator: crate::ast::UnaryOp::Negate, expression, .. } => {
            let (value, _) = integer_literal(expression)?;
            Some((value, true))
        }
        _ => None,
    }
}

fn parse_integer_magnitude(literal: &str) -> Option<u128> {
    literal
        .strip_prefix("0x")
        .or_else(|| literal.strip_prefix("0X"))
        .map_or_else(|| literal.parse().ok(), |digits| u128::from_str_radix(digits, 16).ok())
}

fn unsigned_maximum(width: u8) -> u128 {
    if width == 128 { u128::MAX } else { (1_u128 << width) - 1 }
}

fn signed_literal_fits(magnitude: u128, negative: bool, width: u8) -> bool {
    if width == 128 {
        return if negative { magnitude <= 1_u128 << 127 } else { magnitude <= i128::MAX as u128 };
    }
    let positive_maximum = (1_u128 << (width - 1)) - 1;
    let negative_maximum = 1_u128 << (width - 1);
    if negative { magnitude <= negative_maximum } else { magnitude <= positive_maximum }
}

fn format_literal(literal: &str, negative: bool) -> String {
    if negative { format!("-{literal}") } else { literal.to_owned() }
}
