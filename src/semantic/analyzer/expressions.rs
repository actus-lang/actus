use crate::ast::{BinaryOp, Expr, PrimitiveType, TypeName, UnaryOp, primitive_type};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;
use super::{canonical_type_name, expression_span, try_type_mismatch};

impl Analyzer {
    pub(crate) fn visit_expression_with_expected(
        &mut self,
        expression: &Expr,
        expected: Option<&crate::ast::TypeName>,
    ) -> Result<(), SemanticError> {
        let previous = self.expected_expression_type.take();
        self.expected_expression_type = expected.cloned();
        let result = self.visit_expression(expression);
        let result = result.and_then(|()| {
            if let Some(expected) = expected {
                self.validate_expected_literal(expression, expected)?;
                self.validate_void_expression(expression, expected)?;
            }
            Ok(())
        });
        self.expected_expression_type = previous;
        result
    }

    pub(crate) fn visit_expression(&mut self, expression: &Expr) -> Result<(), SemanticError> {
        self.record_origin(expression);
        match expression {
            Expr::BufferLiteral { length, .. } => self.visit_buffer_literal(length),
            Expr::Identifier { name, span } => self.visit_identifier(name, *span),
            Expr::Binary { left, operator, right, .. } => {
                self.visit_binary_expression(left, operator, right)
            }
            Expr::Grouping { expression, .. } => self.visit_expression(expression),
            Expr::Unary { operator, expression, .. } => {
                self.visit_expression(expression)?;
                self.validate_unary_operator(*operator, expression)
            }
            Expr::Cast { expression, target, span } => {
                self.visit_expression(expression)?;
                self.validate_primitive_cast(expression, target, *span)
            }
            Expr::Borrow { expression, .. } => self.visit_expression(expression),
            Expr::Try { expression, span } => self.visit_try_expression(expression, *span),
            Expr::Call { callee, arguments, span } => self.visit_call(callee, arguments, *span),
            Expr::MethodCall { receiver, method, arguments, span } => {
                self.visit_method_call(receiver, method, arguments, *span)
            }
            Expr::StructLit { name, type_arguments, fields, span } => {
                self.visit_struct_literal(name, type_arguments, fields, *span)
            }
            Expr::FieldAccess { object, field, span } => {
                self.visit_field_access(object, field, *span)
            }
            Expr::Index { target, index, .. } => {
                self.visit_expression(target)?;
                self.visit_expression(index)?;
                self.validate_index_access(target, index).map(|_| ())
            }
            Expr::Case { mode, subject, branches, span } => {
                self.visit_case_expression(*mode, subject, branches, *span)
            }
            Expr::If { .. } => self.visit_if_expression(expression),
            Expr::Integer { .. } | Expr::FloatLiteral { .. } | Expr::StringLiteral { .. } => Ok(()),
        }
    }

    pub(crate) fn validate_index_access(
        &self,
        target: &Expr,
        index: &Expr,
    ) -> Result<TypeName, SemanticError> {
        let target_type = self.resolved_type_name(target).ok_or_else(|| SemanticError {
            kind: SemanticErrorKind::NonIndexableTarget { found: "unknown".to_owned() },
            span: expression_span(target),
        })?;
        if target_type.name == "Buffer" && target_type.arguments.is_empty() {
            self.validate_integer_index(index)?;
            return Ok(TypeName {
                name: "u8".to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span: expression_span(index),
            });
        }
        if target_type.name != "Array" || target_type.arguments.len() != 2 {
            return Err(SemanticError {
                kind: SemanticErrorKind::NonIndexableTarget {
                    found: canonical_type_name(&target_type),
                },
                span: expression_span(target),
            });
        }
        self.validate_integer_index(index)?;
        self.validate_constant_index(index, &target_type.arguments[1])?;
        Ok(target_type.arguments[0].clone())
    }

    pub(crate) fn validate_index_value(
        &self,
        target: &Expr,
        index: &Expr,
        value: &Expr,
    ) -> Result<(), SemanticError> {
        let element = self.validate_index_access(target, index)?;
        let found = self.expression_type_name(value).unwrap_or_else(|| "unknown".to_owned());
        let expected = canonical_type_name(&element);
        if type_names_match(&expected, &found)
            || (primitive_type(&element.name)
                .is_some_and(|primitive| matches!(primitive, PrimitiveType::Integer { .. }))
                && integer_literal(value).is_some())
        {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::IndexedElementTypeMismatch { expected, found },
            span: expression_span(value),
        })
    }

    fn validate_integer_index(&self, index: &Expr) -> Result<(), SemanticError> {
        let found = self.expression_type_name(index).unwrap_or_else(|| "unknown".to_owned());
        if found == "Int"
            || matches!(found.as_str(), "Usize")
            || primitive_type(&found).is_some_and(is_integer_primitive)
        {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidIndexType { found },
            span: expression_span(index),
        })
    }

    fn validate_constant_index(
        &self,
        index: &Expr,
        capacity: &TypeName,
    ) -> Result<(), SemanticError> {
        let Some((literal, negative)) = integer_literal(index) else { return Ok(()) };
        let Some(capacity) = capacity.name.parse::<u128>().ok() else { return Ok(()) };
        let magnitude = parse_integer_magnitude(&literal).unwrap_or(u128::MAX);
        if negative || magnitude >= capacity {
            return Err(SemanticError {
                kind: SemanticErrorKind::IndexOutOfBounds {
                    index: if negative { format!("-{literal}") } else { literal },
                    capacity: capacity.to_string(),
                },
                span: expression_span(index),
            });
        }
        Ok(())
    }

    fn visit_buffer_literal(&mut self, length: &Expr) -> Result<(), SemanticError> {
        self.visit_expression(length)?;
        self.require_buffer_length(length)
    }

    fn visit_identifier(&self, name: &str, span: SourceSpan) -> Result<(), SemanticError> {
        let index = self.binding(name, span)?;
        self.ensure_readable(index, name, span)
    }

    fn visit_binary_expression(
        &mut self,
        left: &Expr,
        operator: &BinaryOp,
        right: &Expr,
    ) -> Result<(), SemanticError> {
        self.visit_expression(left)?;
        self.visit_expression(right)?;
        self.validate_binary_operator(*operator, left, right)
    }

    fn validate_binary_operator(
        &self,
        operator: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<(), SemanticError> {
        let left_type = self.expression_type_name(left).unwrap_or_else(|| "unknown".to_owned());
        let right_type = self.expression_type_name(right).unwrap_or_else(|| "unknown".to_owned());
        let valid = if operator.is_relational() {
            relational_types_match(&left_type, &right_type)
        } else if matches!(operator, BinaryOp::Equals | BinaryOp::NotEquals) {
            equality_types_match(&left_type, &right_type)
        } else if matches!(operator, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
            left_type == "Bool" && right_type == "Bool"
        } else if matches!(operator, BinaryOp::ShiftLeft | BinaryOp::ShiftRight) {
            is_integer_type_name(&left_type) && is_unsigned_integer_type_name(&right_type)
        } else {
            is_integer_type_name(&left_type) && is_integer_type_name(&right_type)
        };
        if valid {
            self.validate_constant_operator(operator, right, &left_type)?;
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::TypeMismatch {
                callee: binary_operator_name(operator).to_owned(),
                parameter: "operands".to_owned(),
                expected: left_type,
                found: right_type,
            },
            span: expression_span(right),
        })
    }

    fn validate_unary_operator(
        &self,
        operator: UnaryOp,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let operand_type =
            self.expression_type_name(expression).unwrap_or_else(|| "unknown".to_owned());
        let valid = match operator {
            UnaryOp::Negate => true,
            UnaryOp::LogicalNot => operand_type == "Bool",
            UnaryOp::BitwiseNot => is_integer_type_name(&operand_type),
        };
        if valid {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::TypeMismatch {
                callee: unary_operator_name(operator).to_owned(),
                parameter: "operand".to_owned(),
                expected: expected_unary_type(operator).to_owned(),
                found: operand_type,
            },
            span: expression_span(expression),
        })
    }

    fn visit_try_expression(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(expression)?;
        self.validate_try_expression(expression, span)
    }

    fn visit_struct_literal(
        &mut self,
        name: &str,
        type_arguments: &[crate::ast::TypeName],
        fields: &[crate::ast::StructFieldInit],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.pack_types.contains_key(name) {
            self.validate_pack_literal(name, fields, span)
        } else {
            self.validate_struct_literal(name, type_arguments, fields, span)
        }
    }

    fn visit_field_access(
        &mut self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.enum_receiver_name(object).is_some() {
            return self.validate_enum_unit_variant(object, field, span);
        }
        self.validate_field_access(object, field, span)?;
        self.ensure_field_access_readable(object, field, span)
    }

    fn visit_case_expression(
        &mut self,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[crate::ast::CaseBranch],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.visit_expression(subject)?;
        self.validate_case_patterns(mode, subject, branches, span)
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

    fn validate_primitive_cast(
        &self,
        expression: &Expr,
        target: &TypeName,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let source = self.expression_type_name(expression).unwrap_or_else(|| "unknown".to_owned());
        if !is_cast_integer_type(&source) || !is_cast_integer_type(&target.name) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidPrimitiveCast {
                    source,
                    target: target.name.clone(),
                },
                span,
            });
        }
        if let Some((literal, negative)) = integer_literal(expression)
            && !cast_literal_fits(&literal, negative, &target.name)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::PrimitiveCastOutOfRange {
                    target: target.name.clone(),
                    literal: if negative { format!("-{literal}") } else { literal },
                },
                span,
            });
        }
        Ok(())
    }
}

fn is_integer_primitive(primitive: PrimitiveType) -> bool {
    matches!(primitive, PrimitiveType::Integer { .. })
}

fn relational_types_match(left: &str, right: &str) -> bool {
    if left == "Int" && right == "Int" {
        return true;
    }
    let left_primitive = if left == "Int" {
        Some(PrimitiveType::Integer { signed: true, width: 32 })
    } else {
        primitive_type(left)
    };
    let right_primitive = if right == "Int" {
        Some(PrimitiveType::Integer { signed: true, width: 32 })
    } else {
        primitive_type(right)
    };
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

fn equality_types_match(left: &str, right: &str) -> bool {
    (left == "Bool" && right == "Bool") || relational_types_match(left, right)
}

fn is_integer_type_name(name: &str) -> bool {
    name == "Int" || name == "Usize" || primitive_type(name).is_some_and(is_integer_primitive)
}

fn is_unsigned_integer_type_name(name: &str) -> bool {
    if name == "Usize" {
        return true;
    }
    primitive_type(name)
        .is_some_and(|primitive| matches!(primitive, PrimitiveType::Integer { signed: false, .. }))
}

fn binary_operator_name(operator: BinaryOp) -> &'static str {
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

fn unary_operator_name(operator: UnaryOp) -> &'static str {
    match operator {
        UnaryOp::LogicalNot => "logical not operator",
        UnaryOp::BitwiseNot => "bitwise not operator",
        UnaryOp::Negate => "negation operator",
    }
}

fn expected_unary_type(operator: UnaryOp) -> &'static str {
    match operator {
        UnaryOp::LogicalNot => "Bool",
        UnaryOp::BitwiseNot => "integer",
        UnaryOp::Negate => "numeric",
    }
}

fn is_cast_integer_type(name: &str) -> bool {
    name == "Int" || name == "Usize" || primitive_type(name).is_some_and(is_integer_primitive)
}

fn cast_literal_fits(literal: &str, negative: bool, target: &str) -> bool {
    let Some(magnitude) = parse_integer_magnitude(literal) else { return false };
    if target == "Int" {
        let positive_limit = (1u128 << 31) - 1;
        let negative_limit = 1u128 << 31;
        return (!negative && magnitude <= positive_limit)
            || (negative && magnitude <= negative_limit);
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

fn type_names_match(expected: &str, found: &str) -> bool {
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
