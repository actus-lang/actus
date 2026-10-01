use crate::ast::{BinaryOp, Expr, PrimitiveType, UnaryOp, primitive_type};

pub(super) fn is_integer_primitive(primitive: PrimitiveType) -> bool {
    matches!(primitive, PrimitiveType::Integer { .. })
}

pub(super) fn relational_types_match(left: &str, right: &str) -> bool {
    let left_primitive = primitive_for_comparison(left);
    let right_primitive = primitive_for_comparison(right);
    match (left_primitive, right_primitive) {
        (
            Some(PrimitiveType::Integer { signed: left_signed, .. }),
            Some(PrimitiveType::Integer { signed: right_signed, .. }),
        ) => left_signed == right_signed,
        (
            Some(PrimitiveType::Float { width: left_width }),
            Some(PrimitiveType::Float { width: right_width }),
        ) => left_width == right_width,
        _ => false,
    }
}

fn primitive_for_comparison(name: &str) -> Option<PrimitiveType> {
    if name == "Int" {
        Some(PrimitiveType::Integer { signed: true, width: 32 })
    } else {
        primitive_type(name)
    }
}

pub(super) fn equality_types_match(left: &str, right: &str) -> bool {
    (left == "Bool" && right == "Bool") || relational_types_match(left, right)
}

pub(super) fn is_integer_type_name(name: &str) -> bool {
    name == "Int" || name == "Usize" || primitive_type(name).is_some_and(is_integer_primitive)
}

pub(super) fn is_unsigned_integer_type_name(name: &str) -> bool {
    name == "Usize"
        || primitive_type(name).is_some_and(|primitive| {
            matches!(primitive, PrimitiveType::Integer { signed: false, .. })
        })
}

pub(super) fn binary_operator_name(operator: BinaryOp) -> &'static str {
    match operator {
        BinaryOp::Equals => "equality operator",
        BinaryOp::NotEquals => "inequality operator",
        BinaryOp::Remainder => "remainder operator",
        BinaryOp::LogicalAnd => "logical and operator",
        BinaryOp::LogicalOr => "logical or operator",
        BinaryOp::BitwiseAnd => "bitwise and operator",
        BinaryOp::BitwiseOr => "bitwise or operator",
        BinaryOp::BitwiseXor => "bitwise xor operator",
        BinaryOp::ShiftLeft => "left shift operator",
        BinaryOp::ShiftRight => "right shift operator",
        _ => "relational operator",
    }
}

pub(super) fn unary_operator_name(operator: UnaryOp) -> &'static str {
    match operator {
        UnaryOp::LogicalNot => "logical not operator",
        UnaryOp::BitwiseNot => "bitwise not operator",
        UnaryOp::Negate => "negation operator",
    }
}

pub(super) fn expected_unary_type(operator: UnaryOp) -> &'static str {
    match operator {
        UnaryOp::LogicalNot => "Bool",
        UnaryOp::BitwiseNot => "integer",
        UnaryOp::Negate => "numeric",
    }
}

pub(super) fn is_cast_integer_type(name: &str) -> bool {
    is_integer_type_name(name)
}

pub(super) fn cast_literal_fits(literal: &str, negative: bool, target: &str) -> bool {
    let Some(magnitude) = parse_integer_magnitude(literal) else { return false };
    if target == "Int" {
        return (!negative && magnitude < (1u128 << 31)) || (negative && magnitude <= 1u128 << 31);
    }
    if target == "Usize" {
        return !negative && magnitude <= u64::MAX as u128;
    }
    let Some(PrimitiveType::Integer { signed, width }) = primitive_type(target) else {
        return true;
    };
    if signed {
        let positive_limit = (1u128 << (width - 1)) - 1;
        let negative_limit = 1u128 << (width - 1);
        (!negative && magnitude <= positive_limit) || (negative && magnitude <= negative_limit)
    } else {
        !negative && (width == 128 || magnitude < (1u128 << width))
    }
}

pub(super) fn integer_literal(expression: &Expr) -> Option<(String, bool)> {
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

pub(super) fn parse_integer_magnitude(literal: &str) -> Option<u128> {
    literal
        .strip_prefix("0x")
        .or_else(|| literal.strip_prefix("0X"))
        .map_or_else(|| literal.parse().ok(), |digits| u128::from_str_radix(digits, 16).ok())
}

pub(super) fn type_names_match(expected: &str, found: &str) -> bool {
    strip_reference_role(expected) == strip_reference_role(found)
}

fn strip_reference_role(type_name: &str) -> &str {
    type_name
        .strip_prefix("abs ")
        .or_else(|| type_name.strip_prefix("ins "))
        .or_else(|| type_name.strip_prefix("erg "))
        .or_else(|| type_name.strip_prefix("dat "))
        .unwrap_or(type_name)
}
