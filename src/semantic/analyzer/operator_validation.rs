use crate::ast::{BinaryOp, Expr, PrimitiveType, UnaryOp, primitive_type};

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::{Analyzer, expression_span};

impl Analyzer {
    pub(super) fn validate_constant_operator(
        &self,
        operator: BinaryOp,
        right: &Expr,
        left_type: &str,
    ) -> Result<(), SemanticError> {
        if matches!(operator, BinaryOp::Remainder) && is_zero_integer(right) {
            return Err(SemanticError {
                kind: SemanticErrorKind::ConstantRemainderByZero,
                span: expression_span(right),
            });
        }
        if let Some(error) = constant_shift_error(operator, right, left_type) {
            return Err(error);
        }
        Ok(())
    }
}

fn constant_shift_error(
    operator: BinaryOp,
    right: &Expr,
    left_type: &str,
) -> Option<SemanticError> {
    if !matches!(operator, BinaryOp::ShiftLeft | BinaryOp::ShiftRight) {
        return None;
    }
    let (literal, negative) = integer_literal(right)?;
    if negative {
        return None;
    }
    let count = parse_integer_magnitude(&literal)?;
    let width = integer_width(left_type)?;
    (count >= u128::from(width)).then(|| SemanticError {
        kind: SemanticErrorKind::ConstantShiftCountOutOfRange { count: literal, width },
        span: expression_span(right),
    })
}

fn is_zero_integer(expression: &Expr) -> bool {
    integer_literal(expression).is_some_and(|(literal, negative)| {
        !negative && parse_integer_magnitude(&literal) == Some(0)
    })
}

fn integer_width(name: &str) -> Option<u16> {
    if name == "Int" {
        return Some(32);
    }
    if name == "Usize" {
        return Some(usize::BITS as u16);
    }
    match primitive_type(name) {
        Some(PrimitiveType::Integer { width, .. }) => Some(u16::from(width)),
        _ => None,
    }
}

fn integer_literal(expression: &Expr) -> Option<(String, bool)> {
    match expression {
        Expr::Integer { value, .. } => Some((value.clone(), false)),
        Expr::Grouping { expression, .. } => integer_literal(expression),
        Expr::Unary { operator: UnaryOp::Negate, expression, .. } => {
            let (value, _) = integer_literal(expression)?;
            Some((value, true))
        }
        Expr::Cast { expression, .. } => integer_literal(expression),
        _ => None,
    }
}

fn parse_integer_magnitude(literal: &str) -> Option<u128> {
    literal
        .strip_prefix("0x")
        .or_else(|| literal.strip_prefix("0X"))
        .map_or_else(|| literal.parse().ok(), |digits| u128::from_str_radix(digits, 16).ok())
}
