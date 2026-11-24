use crate::ast::{BinaryOp, Expr, PrimitiveType, TypeName, UnaryOp, primitive_type};

use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::expressions::rules::{binary_operator_name, is_integer_type_name};
use super::{Analyzer, expression_span};

impl Analyzer {
    /// Both operands of an arithmetic or bitwise operator have one integer
    /// type. An unsuffixed literal on either side takes the other side's type
    /// and must fit it; any other difference is `E1140` (ADR-0119).
    pub(super) fn validate_same_type_operands(
        &self,
        operator: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<(), SemanticError> {
        let left_type = &self.expression_type_name(left).unwrap_or_else(|| "unknown".to_owned());
        let right_type = &self.expression_type_name(right).unwrap_or_else(|| "unknown".to_owned());
        if !(is_integer_type_name(left_type) && is_integer_type_name(right_type)) {
            return Err(SemanticError {
                kind: SemanticErrorKind::TypeMismatch {
                    callee: binary_operator_name(operator).to_owned(),
                    parameter: "operands".to_owned(),
                    expected: left_type.to_owned(),
                    found: right_type.to_owned(),
                },
                span: expression_span(right),
            });
        }
        let mut operand_type = left_type;
        if right.is_plain_integer_literal() {
            self.validate_literal_fits(right, left_type)?;
        } else if left_type != right_type && left.is_plain_integer_literal() {
            self.validate_literal_fits(left, right_type)?;
            operand_type = right_type;
        } else if left_type != right_type {
            return Err(SemanticError {
                kind: SemanticErrorKind::MixedIntegerOperands {
                    operator: operator.symbol().to_owned(),
                    left: left_type.to_owned(),
                    right: right_type.to_owned(),
                },
                span: expression_span(right),
            });
        }
        self.validate_constant_operator(operator, left, right, operand_type)
    }

    /// An unsuffixed literal shift count must fit the shifted type; returns
    /// whether the count was such a literal, which makes the operands valid.
    pub(super) fn validate_literal_shift_count(
        &self,
        operator: BinaryOp,
        count: &Expr,
        shifted_type: &str,
    ) -> Result<bool, SemanticError> {
        let is_literal = matches!(operator, BinaryOp::ShiftLeft | BinaryOp::ShiftRight)
            && is_unsuffixed_integer_literal(count)
            && is_integer_type_name(shifted_type);
        if is_literal {
            self.validate_literal_fits(count, shifted_type)?;
        }
        Ok(is_literal)
    }

    fn validate_literal_fits(&self, literal: &Expr, type_name: &str) -> Result<(), SemanticError> {
        let expected =
            TypeName::builtin(type_name.to_owned(), Vec::new(), None, expression_span(literal));
        self.validate_expected_literal(literal, &expected)
    }

    pub(super) fn validate_constant_operator(
        &self,
        operator: BinaryOp,
        left: &Expr,
        right: &Expr,
        left_type: &str,
    ) -> Result<(), SemanticError> {
        if let Some(error) = constant_overflow_error(operator, left, right, left_type) {
            return Err(error);
        }
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

fn is_unsuffixed_integer_literal(expression: &Expr) -> bool {
    match expression {
        Expr::Integer { suffix: None, .. } => true,
        Expr::Grouping { expression, .. } | Expr::Cast { expression, .. } => {
            is_unsuffixed_integer_literal(expression)
        }
        Expr::Unary { operator: UnaryOp::Negate, expression, .. } => {
            is_unsuffixed_integer_literal(expression)
        }
        _ => false,
    }
}

/// A trapping `+`, `-` or `*` whose operands are integer literals and whose
/// result does not fit the operand type (ADR-0115).
fn constant_overflow_error(
    operator: BinaryOp,
    left: &Expr,
    right: &Expr,
    left_type: &str,
) -> Option<SemanticError> {
    if !matches!(operator, BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply) {
        return None;
    }
    let expression = Expr::Binary {
        left: Box::new(left.clone()),
        operator,
        right: Box::new(right.clone()),
        span: expression_span(right),
    };
    let value = literal_arithmetic_value(&expression)?;
    let (minimum, maximum) = integer_range(left_type)?;
    (value < minimum || value > maximum).then(|| SemanticError {
        kind: SemanticErrorKind::ConstantArithmeticOverflow {
            operator: operator.symbol().to_owned(),
            ty: left_type.to_owned(),
        },
        span: SourceSpan::new(expression_span(left).start, expression_span(right).end),
    })
}

/// The exact value of an expression built from integer literals with `+`,
/// `-`, `*` and negation, or `None` when an operand is not a literal.
fn literal_arithmetic_value(expression: &Expr) -> Option<i128> {
    match expression {
        Expr::Integer { value, .. } => i128::try_from(parse_integer_magnitude(value)?).ok(),
        Expr::Grouping { expression, .. } => literal_arithmetic_value(expression),
        Expr::Unary { operator: UnaryOp::Negate, expression, .. } => {
            literal_arithmetic_value(expression)?.checked_neg()
        }
        Expr::Binary { left, operator, right, .. } => {
            let left = literal_arithmetic_value(left)?;
            let right = literal_arithmetic_value(right)?;
            match operator {
                BinaryOp::Add => left.checked_add(right),
                BinaryOp::Subtract => left.checked_sub(right),
                BinaryOp::Multiply => left.checked_mul(right),
                _ => None,
            }
        }
        _ => None,
    }
}

fn integer_range(name: &str) -> Option<(i128, i128)> {
    let signed = name == "Int"
        || matches!(primitive_type(name), Some(PrimitiveType::Integer { signed: true, .. }));
    let width = u32::from(integer_width(name)?);
    if width >= 128 {
        return signed.then_some((i128::MIN, i128::MAX)).or(Some((0, i128::MAX)));
    }
    Some(if signed {
        (-(1i128 << (width - 1)), (1i128 << (width - 1)) - 1)
    } else {
        (0, (1i128 << width) - 1)
    })
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
