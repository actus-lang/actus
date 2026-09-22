use std::collections::HashSet;

use crate::ast::{
    CaseBody, CaseBranch, EnumPayload, Expr, LiteralPattern, Pattern, PatternBinding, Role,
    VariantPayload,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(super) fn validate_case_patterns(
        &mut self,
        subject: &Expr,
        branches: &[CaseBranch],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_pattern_coverage(subject, branches, span)?;
        let subject_type =
            self.expression_type_name(subject).unwrap_or_else(|| "unknown".to_owned());
        let mut seen = HashSet::new();
        let mut wildcard_seen = false;
        for branch in branches {
            let pattern_name = pattern_name(&branch.pattern);
            if wildcard_seen {
                return Err(unreachable_pattern(pattern_name, pattern_span(&branch.pattern)));
            }
            if is_wildcard(&branch.pattern) {
                wildcard_seen = true;
            } else if !seen.insert(pattern_name.clone()) {
                return Err(duplicate_pattern(pattern_name, pattern_span(&branch.pattern)));
            }
            self.validate_pattern(&branch.pattern, &subject_type, subject)?;
            self.enter_scope(branch.span);
            self.bind_pattern_variables(&branch.pattern)?;
            self.visit_case_body(&branch.body)?;
            self.leave_scope();
        }
        Ok(())
    }

    fn validate_pattern_coverage(
        &self,
        subject: &Expr,
        branches: &[CaseBranch],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(enum_name) = self.expression_enum_type(subject) else {
            if branches.iter().any(|branch| is_wildcard(&branch.pattern)) {
                return Ok(());
            }
            return Err(non_exhaustive("primitive", vec!["_".to_owned()], span));
        };
        if branches.iter().any(|branch| is_wildcard(&branch.pattern)) {
            return Ok(());
        }
        let covered = branches
            .iter()
            .filter_map(|branch| variant_key(&branch.pattern, &enum_name))
            .collect::<HashSet<_>>();
        let missing = self.enum_types[&enum_name]
            .variants
            .iter()
            .filter(|variant| !covered.contains(&variant.name))
            .map(|variant| variant.name.clone())
            .collect::<Vec<_>>();
        if missing.is_empty() { Ok(()) } else { Err(non_exhaustive(&enum_name, missing, span)) }
    }

    fn validate_pattern(
        &self,
        pattern: &Pattern,
        subject_type: &str,
        subject: &Expr,
    ) -> Result<(), SemanticError> {
        match pattern {
            Pattern::Wildcard { .. } => Ok(()),
            Pattern::Literal { value, span } => {
                self.validate_literal_pattern(value, subject_type, *span)
            }
            Pattern::Variant { enum_name, variant, payload, span } => {
                if self.expression_enum_type(subject).as_deref() != Some(enum_name) {
                    return Err(pattern_type_mismatch(subject_type, enum_name, *span));
                }
                let definition = &self.enum_types[enum_name];
                let Some(candidate) = definition.variants.iter().find(|item| item.name == *variant)
                else {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::UnknownEnumVariant {
                            enum_name: enum_name.clone(),
                            variant: variant.clone(),
                        },
                        span: *span,
                    });
                };
                self.validate_variant_payload(candidate.payload.clone(), payload, *span)
            }
        }
    }

    fn validate_literal_pattern(
        &self,
        pattern: &LiteralPattern,
        subject_type: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let pattern_type = match pattern {
            LiteralPattern::Integer(_) => "Int",
            LiteralPattern::Bool(_) => "Bool",
        };
        if pattern_type == subject_type {
            Ok(())
        } else {
            Err(pattern_type_mismatch(subject_type, pattern_type, span))
        }
    }

    fn validate_variant_payload(
        &self,
        payload: EnumPayload,
        pattern: &VariantPayload,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        match (payload, pattern) {
            (EnumPayload::Unit, VariantPayload::Unit) => Ok(()),
            (EnumPayload::Tuple(types), VariantPayload::Positional(bindings)) => {
                if types.len() != bindings.len() {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::EnumVariantArgumentCount {
                            enum_name: "pattern".to_owned(),
                            variant: "payload".to_owned(),
                            expected: types.len(),
                            found: bindings.len(),
                        },
                        span,
                    });
                }
                Ok(())
            }
            (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => {
                if fields.len() != patterns.len() {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::EnumVariantArgumentCount {
                            enum_name: "pattern".to_owned(),
                            variant: "payload".to_owned(),
                            expected: fields.len(),
                            found: patterns.len(),
                        },
                        span,
                    });
                }
                let mut names = HashSet::new();
                for item in patterns {
                    if !names.insert(item.name.clone()) {
                        return Err(duplicate_pattern(item.name.clone(), item.span));
                    }
                    if !fields.iter().any(|field| field.name == item.name) {
                        return Err(SemanticError {
                            kind: SemanticErrorKind::EnumVariantArgumentName {
                                enum_name: "pattern".to_owned(),
                                variant: "payload".to_owned(),
                                name: item.name.clone(),
                            },
                            span: item.span,
                        });
                    }
                }
                Ok(())
            }
            _ => Err(pattern_type_mismatch("matching payload", "different payload", span)),
        }
    }

    fn bind_pattern_variables(&mut self, pattern: &Pattern) -> Result<(), SemanticError> {
        match pattern {
            Pattern::Variant { enum_name, variant, payload, .. } => {
                let candidate_payload = self.enum_types[enum_name]
                    .variants
                    .iter()
                    .find(|item| item.name == *variant)
                    .unwrap()
                    .payload
                    .clone();
                match (candidate_payload, payload) {
                    (EnumPayload::Tuple(types), VariantPayload::Positional(bindings)) => {
                        for (binding, ty) in bindings.iter().zip(types) {
                            self.bind_pattern_binding(binding, &ty.name)?;
                        }
                    }
                    (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => {
                        for pattern in patterns {
                            let field =
                                fields.iter().find(|field| field.name == pattern.name).unwrap();
                            self.bind_pattern_binding(&pattern.binding, &field.ty.name)?;
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            Pattern::Literal { .. } | Pattern::Wildcard { .. } => Ok(()),
        }
    }

    fn bind_pattern_binding(
        &mut self,
        binding: &PatternBinding,
        type_name: &str,
    ) -> Result<(), SemanticError> {
        if binding.name == "_" {
            return Ok(());
        }
        self.validate_type_name(type_name, binding.span)?;
        self.bind(
            Role::Abs,
            binding.name.clone(),
            crate::ast::lookup_builtin_type(type_name),
            binding.span,
        )?;
        let index = self.binding(&binding.name, binding.span)?;
        if self.struct_types.contains_key(type_name) {
            self.binding_struct_types.insert(index, type_name.to_owned());
        }
        if self.enum_types.contains_key(type_name) {
            self.binding_enum_types.insert(index, type_name.to_owned());
        }
        Ok(())
    }

    fn visit_case_body(&mut self, body: &CaseBody) -> Result<(), SemanticError> {
        match body {
            CaseBody::Expression(expression) => self.visit_expression(expression),
            CaseBody::Block(block) => self.visit_block(block),
        }
    }
}

fn pattern_span(pattern: &Pattern) -> SourceSpan {
    match pattern {
        Pattern::Variant { span, .. }
        | Pattern::Literal { span, .. }
        | Pattern::Wildcard { span } => *span,
    }
}

fn pattern_name(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Variant { enum_name, variant, .. } => format!("{enum_name}.{variant}"),
        Pattern::Literal { value, .. } => match value {
            LiteralPattern::Integer(value) => value.clone(),
            LiteralPattern::Bool(value) => value.to_string(),
        },
        Pattern::Wildcard { .. } => "_".to_owned(),
    }
}

fn variant_key(pattern: &Pattern, enum_name: &str) -> Option<String> {
    match pattern {
        Pattern::Variant { enum_name: found, variant, .. } if found == enum_name => {
            Some(variant.clone())
        }
        _ => None,
    }
}

fn is_wildcard(pattern: &Pattern) -> bool {
    matches!(pattern, Pattern::Wildcard { .. })
}

fn non_exhaustive(subject: &str, missing: Vec<String>, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::NonExhaustiveMatch { subject: subject.to_owned(), missing },
        span,
    }
}

fn unreachable_pattern(pattern: String, span: SourceSpan) -> SemanticError {
    SemanticError { kind: SemanticErrorKind::UnreachablePattern { pattern }, span }
}

fn duplicate_pattern(pattern: String, span: SourceSpan) -> SemanticError {
    SemanticError { kind: SemanticErrorKind::DuplicatePattern { pattern }, span }
}

fn pattern_type_mismatch(expected: &str, found: &str, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::PatternTypeMismatch {
            expected: expected.to_owned(),
            found: found.to_owned(),
        },
        span,
    }
}
