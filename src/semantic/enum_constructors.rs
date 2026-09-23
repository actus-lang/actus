use std::collections::HashSet;

use crate::ast::{Argument, EnumDef, EnumPayload, EnumVariant, Expr, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::type_substitution::TypeSubstitution;

impl Analyzer {
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
        if let Some(receiver_type) = receiver_type_name(receiver) {
            self.validate_type_reference(&receiver_type)?;
        }
        let substitution = enum_substitution(receiver, &definition)?;
        let Some(candidate) = find_variant(&definition, variant).cloned() else {
            return Err(unknown_variant(&enum_name, variant, span));
        };
        match &candidate.payload {
            EnumPayload::Unit => {
                self.validate_variant_count(&enum_name, variant, 0, arguments, span)
            }
            EnumPayload::Tuple(types) => self.validate_tuple_constructor(
                &enum_name,
                variant,
                types,
                substitution.as_ref(),
                arguments,
                span,
            ),
            EnumPayload::Struct(fields) => self.validate_named_constructor(
                &enum_name,
                variant,
                fields,
                substitution.as_ref(),
                arguments,
                span,
            ),
        }
    }

    fn validate_tuple_constructor(
        &mut self,
        enum_name: &str,
        variant: &str,
        types: &[TypeName],
        substitution: Option<&TypeSubstitution>,
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
            let expected = substitution
                .map(|substitution| substitution.apply(&types[index]))
                .unwrap_or_else(|| types[index].clone());
            self.validate_variant_argument(
                enum_name,
                variant,
                &index.to_string(),
                &expected,
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
        substitution: Option<&TypeSubstitution>,
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
                return Err(named_argument_error(enum_name, variant, name, argument));
            }
            let Some(field) = fields.iter().find(|field| field.name == *name) else {
                return Err(named_argument_error(enum_name, variant, name, argument));
            };
            let expected = substitution
                .map(|substitution| substitution.apply(&field.ty))
                .unwrap_or_else(|| field.ty.clone());
            self.validate_variant_argument(enum_name, variant, name, &expected, argument)?;
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
        expected: &TypeName,
        argument: &Argument,
    ) -> Result<(), SemanticError> {
        self.visit_expression(&argument.expression)?;
        let found =
            self.expression_type_name(&argument.expression).unwrap_or_else(|| "unknown".to_owned());
        if found == canonical_type_name(expected) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::EnumVariantArgumentTypeMismatch {
                variant: variant.to_owned(),
                parameter: parameter.to_owned(),
                expected: canonical_type_name(expected),
                found,
            },
            span: argument_span(argument),
        })
    }
}

fn named_argument_error(
    enum_name: &str,
    variant: &str,
    name: &str,
    argument: &Argument,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::EnumVariantArgumentName {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
            name: name.to_owned(),
        },
        span: argument_span(argument),
    }
}

fn find_variant<'a>(definition: &'a EnumDef, name: &str) -> Option<&'a EnumVariant> {
    definition.variants.iter().find(|variant| variant.name == name)
}

pub(super) fn unknown_variant(enum_name: &str, variant: &str, span: SourceSpan) -> SemanticError {
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
        | Expr::FieldAccess { span, .. }
        | Expr::Case { span, .. } => *span,
    }
}

fn enum_substitution(
    receiver: &Expr,
    definition: &EnumDef,
) -> Result<Option<TypeSubstitution>, SemanticError> {
    if definition.generic_parameters.is_empty() {
        return Ok(None);
    }
    let type_name = receiver_type_name(receiver).ok_or_else(|| SemanticError {
        kind: SemanticErrorKind::UnknownType { name: definition.name.clone() },
        span: definition.span,
    })?;
    TypeSubstitution::for_type(
        &definition.name,
        &definition.generic_parameters,
        &type_name.arguments,
        type_name.span,
    )
    .map(Some)
}

fn receiver_type_name(receiver: &Expr) -> Option<TypeName> {
    let Expr::Identifier { name, span } = receiver else { return None };
    parse_type_name_key(name, *span)
}

fn parse_type_name_key(key: &str, span: SourceSpan) -> Option<TypeName> {
    let Some(open) = key.find('[') else {
        return Some(TypeName { name: key.to_owned(), arguments: Vec::new(), span });
    };
    if !key.ends_with(']') {
        return None;
    }
    let arguments = split_type_arguments(&key[open + 1..key.len() - 1])
        .into_iter()
        .map(|argument| parse_type_name_key(argument, span))
        .collect::<Option<Vec<_>>>()?;
    Some(TypeName { name: key[..open].to_owned(), arguments, span })
}

fn split_type_arguments(contents: &str) -> Vec<&str> {
    let mut depth = 0;
    let mut start = 0;
    let mut arguments = Vec::new();
    for (index, character) in contents.char_indices() {
        match character {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                arguments.push(contents[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < contents.len() {
        arguments.push(contents[start..].trim());
    }
    arguments
}

fn canonical_type_name(type_name: &TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

pub(super) fn generic_receiver_type(receiver: &Expr) -> Option<TypeName> {
    receiver_type_name(receiver)
}
