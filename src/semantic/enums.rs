use std::collections::HashSet;

use crate::ast::{Argument, EnumDef, EnumPayload, EnumVariant, Expr};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn enum_receiver_name<'a>(&self, expression: &'a Expr) -> Option<&'a str> {
        let Expr::Identifier { name, .. } = expression else { return None };
        self.enum_types.contains_key(name).then_some(name.as_str())
    }

    pub(super) fn validate_enum_unit_variant(
        &self,
        receiver: &Expr,
        variant: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let enum_name = self.enum_receiver_name(receiver).expect("enum receiver expected");
        let definition = &self.enum_types[enum_name];
        let Some(candidate) = find_variant(definition, variant) else {
            return Err(unknown_variant(enum_name, variant, span));
        };
        if !matches!(candidate.payload, EnumPayload::Unit) {
            return Err(argument_count(
                enum_name,
                variant,
                payload_count(&candidate.payload),
                0,
                span,
            ));
        }
        Ok(())
    }

    pub(super) fn validate_enum_constructor(
        &mut self,
        receiver: &Expr,
        variant: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let enum_name =
            self.enum_receiver_name(receiver).expect("enum receiver expected").to_owned();
        let definition = self.enum_types[&enum_name].clone();
        let Some(candidate) = find_variant(&definition, variant).cloned() else {
            return Err(unknown_variant(&enum_name, variant, span));
        };
        match &candidate.payload {
            EnumPayload::Unit => {
                self.validate_variant_count(&enum_name, variant, 0, arguments, span)
            }
            EnumPayload::Tuple(types) => {
                self.validate_tuple_constructor(&enum_name, variant, types, arguments, span)
            }
            EnumPayload::Struct(fields) => {
                self.validate_named_constructor(&enum_name, variant, fields, arguments, span)
            }
        }
    }

    fn validate_tuple_constructor(
        &mut self,
        enum_name: &str,
        variant: &str,
        types: &[crate::ast::TypeName],
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_variant_count(enum_name, variant, types.len(), arguments, span)?;
        for (index, argument) in arguments.iter().enumerate() {
            if let Some(name) = &argument.name {
                return Err(SemanticError {
                    kind: SemanticErrorKind::EnumVariantArgumentName {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                        name: name.clone(),
                    },
                    span: argument_span(argument),
                });
            }
            self.validate_variant_argument(
                enum_name,
                variant,
                &index.to_string(),
                &types[index].name,
                argument,
            )?;
        }
        Ok(())
    }

    fn validate_named_constructor(
        &mut self,
        enum_name: &str,
        variant: &str,
        fields: &[crate::ast::EnumField],
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_variant_count(enum_name, variant, fields.len(), arguments, span)?;
        let mut seen = HashSet::new();
        for argument in arguments {
            let Some(name) = &argument.name else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::EnumVariantArgumentName {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                        name: "<positional>".to_owned(),
                    },
                    span: argument_span(argument),
                });
            };
            if !seen.insert(name.clone()) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::EnumVariantArgumentName {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                        name: name.clone(),
                    },
                    span: argument_span(argument),
                });
            }
            let Some(field) = fields.iter().find(|field| field.name == *name) else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::EnumVariantArgumentName {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                        name: name.clone(),
                    },
                    span: argument_span(argument),
                });
            };
            self.validate_variant_argument(enum_name, variant, name, &field.ty.name, argument)?;
        }
        Ok(())
    }

    fn validate_variant_count(
        &self,
        enum_name: &str,
        variant: &str,
        expected: usize,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if expected == arguments.len() {
            return Ok(());
        }
        Err(argument_count(enum_name, variant, expected, arguments.len(), span))
    }

    fn validate_variant_argument(
        &mut self,
        _enum_name: &str,
        variant: &str,
        parameter: &str,
        expected: &str,
        argument: &Argument,
    ) -> Result<(), SemanticError> {
        self.visit_expression(&argument.expression)?;
        let found =
            self.expression_type_name(&argument.expression).unwrap_or_else(|| "unknown".to_owned());
        if found == expected {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::EnumVariantArgumentTypeMismatch {
                variant: variant.to_owned(),
                parameter: parameter.to_owned(),
                expected: expected.to_owned(),
                found,
            },
            span: argument_span(argument),
        })
    }

    pub(super) fn expression_enum_type(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::FieldAccess { object, .. } => {
                let enum_name = self.enum_receiver_name(object)?;
                Some(enum_name.to_owned())
            }
            Expr::MethodCall { receiver, .. } => {
                let enum_name = self.enum_receiver_name(receiver)?;
                Some(enum_name.to_owned())
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
        type_name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if !self.enum_types.contains_key(type_name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_enum_types.insert(index, type_name.to_owned());
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

fn find_variant<'a>(definition: &'a EnumDef, name: &str) -> Option<&'a EnumVariant> {
    definition.variants.iter().find(|variant| variant.name == name)
}

fn payload_count(payload: &EnumPayload) -> usize {
    match payload {
        EnumPayload::Unit => 0,
        EnumPayload::Tuple(types) => types.len(),
        EnumPayload::Struct(fields) => fields.len(),
    }
}

fn unknown_variant(enum_name: &str, variant: &str, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::UnknownEnumVariant {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
        },
        span,
    }
}

fn argument_count(
    enum_name: &str,
    variant: &str,
    expected: usize,
    found: usize,
    span: SourceSpan,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::EnumVariantArgumentCount {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
            expected,
            found,
        },
        span,
    }
}

fn argument_span(argument: &Argument) -> SourceSpan {
    match &argument.expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. } => *span,
    }
}
