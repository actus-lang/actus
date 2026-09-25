use crate::ast::{BuiltinType, Expr, IntrinsicKind, lookup_builtin_type, lookup_call_intrinsic};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn validate_type_name(
        &self,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if lookup_builtin_type(name).is_some()
            || self.struct_types.contains_key(name)
            || self.enum_types.contains_key(name)
        {
            return Ok(());
        }
        Err(SemanticError { kind: SemanticErrorKind::UnknownType { name: name.to_owned() }, span })
    }

    pub(super) fn resolve_binding_type(
        &self,
        declared_type: Option<&str>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<Option<BuiltinType>, SemanticError> {
        if let Some(name) = declared_type {
            self.validate_type_name(name, span)?;
            return Ok(lookup_builtin_type(name));
        }
        Ok(self.expression_type(initializer))
    }

    pub(super) fn validate_declared_initializer(
        &self,
        name: &str,
        declared_type: Option<&str>,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expected_name) = declared_type else { return Ok(()) };
        if self.struct_types.contains_key(expected_name) {
            let found =
                self.expression_struct_type(initializer).unwrap_or_else(|| "unknown".to_owned());
            if found != expected_name {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BindingTypeMismatch {
                        binding: name.to_owned(),
                        expected: expected_name.to_owned(),
                        found,
                    },
                    span,
                });
            }
            return Ok(());
        }
        if self.enum_types.contains_key(expected_name) {
            let found =
                self.expression_type_name(initializer).unwrap_or_else(|| "unknown".to_owned());
            if found != expected_name {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BindingTypeMismatch {
                        binding: name.to_owned(),
                        expected: expected_name.to_owned(),
                        found,
                    },
                    span,
                });
            }
            return Ok(());
        }
        let Some(found) = self.expression_type(initializer) else { return Ok(()) };
        let expected = lookup_builtin_type(expected_name).expect("declared type was validated");
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
            Expr::Binary { .. } => Some(BuiltinType::Int),
            Expr::Identifier { name, span } => {
                self.binding(name, *span).ok().and_then(|index| self.model.bindings[index].ty)
            }
            Expr::Call { callee, .. } => match lookup_call_intrinsic(callee) {
                Some(IntrinsicKind::Append) => Some(BuiltinType::Int),
                Some(IntrinsicKind::Print) => Some(BuiltinType::Int),
                Some(IntrinsicKind::Drop) => None,
                None => self.signatures.get(callee).and_then(|signature| signature.return_type),
            },
            Expr::MethodCall { receiver, method, .. } if method == "raw_slice" => {
                (self.expression_type(receiver) == Some(BuiltinType::Buffer))
                    .then_some(BuiltinType::Buffer)
            }
            Expr::MethodCall { method, .. } => {
                self.signatures.get(method).and_then(|signature| signature.return_type)
            }
            Expr::StructLit { .. } => None,
            Expr::FieldAccess { object, field, .. } => self
                .expression_struct_type(object)
                .and_then(|name| self.struct_field(&name, field))
                .and_then(|field| lookup_builtin_type(&field.ty.name)),
            Expr::Case { .. } => None,
        }
    }

    pub(super) fn expression_type_name(&self, expression: &Expr) -> Option<String> {
        self.expression_type(expression)
            .map(|ty| ty.spec().name.to_owned())
            .or_else(|| self.expression_struct_type(expression))
            .or_else(|| self.expression_enum_type_application(expression))
            .or_else(|| self.expression_enum_type(expression))
    }
}
