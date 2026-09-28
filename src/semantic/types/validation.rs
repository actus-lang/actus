use crate::ast::{BuiltinType, Expr, PrimitiveType, TypeName, lookup_builtin_type, primitive_type};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(crate) fn validate_void_expression(
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

    pub(crate) fn validate_type_name(
        &self,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if name == "Arena"
            || self.type_registry.is_known(name)
            || self.struct_types.contains_key(name)
            || self.pack_types.contains_key(name)
            || self.enum_types.contains_key(name)
        {
            return Ok(());
        }
        Err(SemanticError { kind: SemanticErrorKind::UnknownType { name: name.to_owned() }, span })
    }

    pub(crate) fn resolve_binding_type(
        &mut self,
        declared_type: Option<&TypeName>,
        initializer: &Expr,
        _span: SourceSpan,
    ) -> Result<Option<BuiltinType>, SemanticError> {
        if let Some(type_name) = declared_type {
            self.validate_type_reference(type_name)?;
            return Ok(lookup_builtin_type(&type_name.name));
        }
        Ok(self.expression_type(initializer))
    }

    pub(crate) fn validate_declared_initializer(
        &self,
        name: &str,
        declared_type: Option<&TypeName>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expected_type) = declared_type else { return Ok(()) };
        let expected_name = crate::semantic::analyzer::canonical_type_name(expected_type);
        if self.struct_types.contains_key(&expected_type.name)
            || self.pack_types.contains_key(&expected_type.name)
        {
            return self.validate_aggregate_initializer(name, &expected_name, initializer, span);
        }
        if self.enum_types.contains_key(&expected_type.name) {
            return self.validate_enum_initializer(name, &expected_name, initializer, span);
        }
        self.validate_scalar_initializer(name, expected_type, &expected_name, initializer, span)
    }

    fn validate_aggregate_initializer(
        &self,
        name: &str,
        expected: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let found = self
            .expression_struct_type(initializer)
            .or_else(|| self.expression_pack_type(initializer))
            .unwrap_or_else(|| "unknown".to_owned());
        self.ensure_named_binding_type(name, expected, found, span)
    }

    fn validate_enum_initializer(
        &self,
        name: &str,
        expected: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let found = self.expression_type_name(initializer).unwrap_or_else(|| "unknown".to_owned());
        self.ensure_named_binding_type(name, expected, found, span)
    }

    fn validate_scalar_initializer(
        &self,
        name: &str,
        expected_type: &TypeName,
        expected: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_expected_literal(initializer, expected_type)?;
        if primitive_type(&expected_type.name).is_some() {
            return self.validate_named_value_type(name, expected_type, initializer, span);
        }
        let found = self.expression_type_name(initializer).unwrap_or_else(|| "unknown".to_owned());
        self.ensure_named_binding_type(name, expected, found, span)
    }

    fn ensure_named_binding_type(
        &self,
        name: &str,
        expected: &str,
        found: String,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if type_names_match(expected, &found) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::BindingTypeMismatch {
                binding: name.to_owned(),
                expected: expected.to_owned(),
                found,
            },
            span,
        })
    }

    pub(crate) fn validate_binding_assignment(
        &self,
        index: usize,
        name: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Some(expected) = self.named_binding_type(index) {
            return self.validate_named_value_type(name, &expected, value, span);
        }
        let Some(expected) = self.model.bindings[index].ty else { return Ok(()) };
        let Some(found) = self.expression_type(value) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::BindingTypeMismatch {
                    binding: name.to_owned(),
                    expected: expected.spec().name.to_owned(),
                    found: "unknown".to_owned(),
                },
                span,
            });
        };
        self.ensure_binding_type(name, expected, found, span)
    }

    fn named_binding_type(&self, index: usize) -> Option<TypeName> {
        if let Some(type_name) = self.binding_type_names.get(&index) {
            return Some(type_name.clone());
        }
        if let Some(type_name) = self.binding_struct_type_applications.get(&index) {
            return Some(type_name.clone());
        }
        if let Some(type_name) = self.binding_enum_type_applications.get(&index) {
            return Some(type_name.clone());
        }
        let name = self
            .binding_struct_types
            .get(&index)
            .or_else(|| self.binding_enum_types.get(&index))?;
        Some(TypeName {
            name: name.clone(),
            arguments: Vec::new(),
            reference_role: None,
            span: SourceSpan::new(0, 0),
        })
    }

    fn validate_named_value_type(
        &self,
        binding: &str,
        expected: &TypeName,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let expected_name = super::super::analyzer::canonical_type_name(expected);
        let found = self.expression_type_name(value).unwrap_or_else(|| "unknown".to_owned());
        let literal_matches = match primitive_type(&expected.name) {
            Some(PrimitiveType::Integer { .. }) => integer_literal(value).is_some(),
            Some(PrimitiveType::Float { .. }) => matches!(value, Expr::FloatLiteral { .. }),
            Some(PrimitiveType::Void) | None => false,
        };
        if literal_matches || type_names_match(&expected_name, &found) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::BindingTypeMismatch {
                binding: binding.to_owned(),
                expected: expected_name,
                found,
            },
            span,
        })
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
