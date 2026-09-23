use crate::ast::{EnumPayload, Expr, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

fn receiver_name<'a>(analyzer: &Analyzer, expression: &'a Expr) -> Option<&'a str> {
    let Expr::Identifier { name, .. } = expression else { return None };
    let base_name = name.split('[').next().unwrap_or(name);
    analyzer.enum_types.contains_key(base_name).then_some(base_name)
}

impl Analyzer {
    pub(super) fn enum_receiver_name<'a>(&self, expression: &'a Expr) -> Option<&'a str> {
        receiver_name(self, expression)
    }

    pub(super) fn validate_enum_unit_variant(
        &mut self,
        receiver: &Expr,
        variant: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let enum_name = self.enum_receiver_name(receiver).expect("enum receiver expected");
        if let Some(receiver_type) = super::enum_constructors::generic_receiver_type(receiver) {
            self.validate_type_reference(&receiver_type)?;
        }
        let definition = &self.enum_types[enum_name];
        let Some(candidate) = definition.variants.iter().find(|item| item.name == variant) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownEnumVariant {
                    enum_name: enum_name.to_owned(),
                    variant: variant.to_owned(),
                },
                span,
            });
        };
        if !matches!(candidate.payload, EnumPayload::Unit) {
            return Err(SemanticError {
                kind: SemanticErrorKind::EnumVariantArgumentCount {
                    enum_name: enum_name.to_owned(),
                    variant: variant.to_owned(),
                    expected: payload_count(&candidate.payload),
                    found: 0,
                },
                span,
            });
        }
        Ok(())
    }

    pub(super) fn expression_enum_type(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::FieldAccess { object, .. } | Expr::MethodCall { receiver: object, .. } => {
                self.enum_receiver_name(object).map(str::to_owned)
            }
            Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
                self.expression_enum_type(expression)
            }
            Expr::Identifier { name, span } => self
                .binding(name, *span)
                .ok()
                .and_then(|index| self.binding_enum_types.get(&index).cloned()),
            _ => None,
        }
    }

    pub(super) fn record_enum_binding(
        &mut self,
        name: &str,
        type_name: &TypeName,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if !self.enum_types.contains_key(&type_name.name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_enum_types.insert(index, type_name.name.clone());
        if !type_name.arguments.is_empty() {
            self.binding_enum_type_applications.insert(index, type_name.clone());
        }
        Ok(())
    }

    pub(super) fn record_initializer_enum_type(
        &mut self,
        name: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(type_name) = self.expression_enum_type(initializer) else { return Ok(()) };
        let index = self.binding(name, span)?;
        self.binding_enum_types.insert(index, type_name);
        Ok(())
    }
}

fn payload_count(payload: &EnumPayload) -> usize {
    match payload {
        EnumPayload::Unit => 0,
        EnumPayload::Tuple(types) => types.len(),
        EnumPayload::Struct(fields) => fields.len(),
    }
}
