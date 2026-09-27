use crate::ast::{
    BuiltinType, CaseBody, Expr, IntrinsicKind, PrimitiveType, TypeName, lookup_builtin_type,
    lookup_call_intrinsic, primitive_type,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticType {
    Builtin(BuiltinType),
    Primitive(PrimitiveType),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TypeRegistry;

impl TypeRegistry {
    pub const fn new() -> Self {
        Self
    }

    pub fn resolve(&self, name: &str) -> Option<SemanticType> {
        primitive_type(name)
            .map(SemanticType::Primitive)
            .or_else(|| lookup_builtin_type(name).map(SemanticType::Builtin))
    }

    pub fn is_known(&self, name: &str) -> bool {
        self.resolve(name).is_some()
    }
}

impl Analyzer {
    pub(super) fn validate_expected_literal(
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

    pub(super) fn validate_void_expression(
        &self,
        expression: &Expr,
        expected: &TypeName,
    ) -> Result<(), SemanticError> {
        if primitive_type(&expected.name) != Some(PrimitiveType::Void) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::ReturnTypeMismatch {
                expected: "Void".to_owned(),
                found: self.expression_type_name(expression).unwrap_or_else(|| "value".to_owned()),
            },
            span: expected.span,
        })
    }

    pub(super) fn validate_type_name(
        &self,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.type_registry.is_known(name)
            || self.struct_types.contains_key(name)
            || self.pack_types.contains_key(name)
            || self.enum_types.contains_key(name)
        {
            return Ok(());
        }
        Err(SemanticError { kind: SemanticErrorKind::UnknownType { name: name.to_owned() }, span })
    }

    pub(super) fn resolve_binding_type(
        &mut self,
        declared_type: Option<&crate::ast::TypeName>,
        initializer: &Expr,
        _span: SourceSpan,
    ) -> Result<Option<BuiltinType>, SemanticError> {
        if let Some(type_name) = declared_type {
            self.validate_type_reference(type_name)?;
            return Ok(lookup_builtin_type(&type_name.name));
        }
        Ok(self.expression_type(initializer))
    }

    pub(super) fn validate_declared_initializer(
        &self,
        name: &str,
        declared_type: Option<&crate::ast::TypeName>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expected_type) = declared_type else { return Ok(()) };
        let expected_name = crate::semantic::analyzer::canonical_type_name(expected_type);
        if self.struct_types.contains_key(&expected_type.name)
            || self.pack_types.contains_key(&expected_type.name)
        {
            let found = self
                .expression_struct_type(initializer)
                .or_else(|| self.expression_pack_type(initializer))
                .unwrap_or_else(|| "unknown".to_owned());
            if found != expected_name {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BindingTypeMismatch {
                        binding: name.to_owned(),
                        expected: expected_name.clone(),
                        found,
                    },
                    span,
                });
            }
            return Ok(());
        }
        if self.enum_types.contains_key(&expected_type.name) {
            let found =
                self.expression_type_name(initializer).unwrap_or_else(|| "unknown".to_owned());
            if found != expected_name {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BindingTypeMismatch {
                        binding: name.to_owned(),
                        expected: expected_name.clone(),
                        found,
                    },
                    span,
                });
            }
            return Ok(());
        }
        self.validate_expected_literal(initializer, expected_type)?;
        if primitive_type(&expected_type.name).is_some() {
            return Ok(());
        }
        let Some(found) = self.expression_type(initializer) else { return Ok(()) };
        let expected =
            lookup_builtin_type(&expected_type.name).expect("declared type was validated");
        self.ensure_binding_type(name, expected, found, span)
    }

    pub(super) fn validate_binding_assignment(
        &self,
        index: usize,
        name: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Some(expected) = self.binding_enum_types.get(&index) {
            let found = self.expression_type_name(value).unwrap_or_else(|| "unknown".to_owned());
            if &found == expected {
                return Ok(());
            }
            return Err(SemanticError {
                kind: SemanticErrorKind::BindingTypeMismatch {
                    binding: name.to_owned(),
                    expected: expected.clone(),
                    found,
                },
                span,
            });
        }
        let Some(expected) = self.model.bindings[index].ty else { return Ok(()) };
        let Some(found) = self.expression_type(value) else { return Ok(()) };
        self.ensure_binding_type(name, expected, found, span)
    }

    fn ensure_binding_type(
        &self,
        name: &str,
        expected: BuiltinType,
        found: BuiltinType,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if expected == found {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::BindingTypeMismatch {
                binding: name.to_owned(),
                expected: expected.spec().name.to_owned(),
                found: found.spec().name.to_owned(),
            },
            span,
        })
    }

    pub(super) fn expression_type(&self, expression: &Expr) -> Option<BuiltinType> {
        match expression {
            Expr::Integer { .. } => Some(BuiltinType::Int),
            Expr::BufferLiteral { .. } => Some(BuiltinType::Buffer),
            Expr::FloatLiteral { .. } => None,
            Expr::StringLiteral { .. } => Some(BuiltinType::String),
            Expr::Grouping { expression, .. }
            | Expr::Borrow { expression, .. }
            | Expr::Unary { expression, .. } => self.expression_type(expression),
            Expr::Try { expression, .. } => self
                .enum_type_application(expression)
                .and_then(|type_name| type_name.arguments.first().cloned())
                .and_then(|type_name| lookup_builtin_type(&type_name.name)),
            Expr::Binary { .. } => Some(BuiltinType::Int),
            Expr::Identifier { name, span } => {
                self.binding(name, *span).ok().and_then(|index| self.model.bindings[index].ty)
            }
            Expr::Call { callee, span, .. } => match lookup_call_intrinsic(callee) {
                Some(IntrinsicKind::Append) => Some(BuiltinType::Int),
                Some(IntrinsicKind::Print) => Some(BuiltinType::Int),
                Some(IntrinsicKind::Drop) => None,
                None => self
                    .inferred_expression_types
                    .get(&(span.start, span.end))
                    .and_then(|type_name| lookup_builtin_type(&type_name.name))
                    .or_else(|| {
                        self.signatures.get(callee).and_then(|signature| signature.return_type)
                    }),
            },
            Expr::MethodCall { receiver, method, .. } if method == "raw_slice" => {
                (self.expression_type(receiver) == Some(BuiltinType::Buffer))
                    .then_some(BuiltinType::Buffer)
            }
            Expr::MethodCall { method, span, .. } => self
                .inferred_expression_types
                .get(&(span.start, span.end))
                .and_then(|type_name| lookup_builtin_type(&type_name.name))
                .or_else(|| {
                    self.signatures.get(method).and_then(|signature| signature.return_type)
                }),
            Expr::StructLit { .. } => None,
            Expr::FieldAccess { object, field, .. } => self
                .expression_struct_type(object)
                .and_then(|name| self.struct_field(&name, field))
                .and_then(|field| lookup_builtin_type(&field.ty.name)),
            Expr::Case { branches, .. } => branches.iter().find_map(|branch| match &branch.body {
                CaseBody::Expression(expression) => self.expression_type(expression),
                CaseBody::Block(_) => None,
            }),
        }
    }

    pub(super) fn expression_type_name(&self, expression: &Expr) -> Option<String> {
        if let Expr::Call { span, .. } | Expr::MethodCall { span, .. } = expression
            && let Some(type_name) = self.inferred_expression_types.get(&(span.start, span.end))
        {
            return Some(super::analyzer::canonical_type_name(type_name));
        }
        self.expression_type(expression)
            .map(|ty| ty.spec().name.to_owned())
            .or_else(|| self.binding_type_name(expression))
            .or_else(|| self.expression_struct_type(expression))
            .or_else(|| self.expression_pack_type(expression))
            .or_else(|| {
                self.resolved_type_name(expression)
                    .map(|name| super::analyzer::canonical_type_name(&name))
            })
            .or_else(|| self.expression_enum_type_application(expression))
            .or_else(|| self.expression_case_type_name(expression))
            .or_else(|| self.expression_enum_type(expression))
    }

    fn binding_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Identifier { name, span } = expression else { return None };
        let index = self.binding(name, *span).ok()?;
        self.binding_type_names.get(&index).map(super::analyzer::canonical_type_name)
    }

    fn expression_case_type_name(&self, expression: &Expr) -> Option<String> {
        let Expr::Case { branches, .. } = expression else { return None };
        branches.iter().find_map(|branch| match &branch.body {
            CaseBody::Expression(expression) => self.expression_type_name(expression),
            CaseBody::Block(_) => None,
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
