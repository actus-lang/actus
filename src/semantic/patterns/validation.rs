use std::collections::HashSet;

use crate::ast::{CaseBranch, EnumPayload, Expr, LiteralPattern, Pattern, VariantPayload};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::super::pattern_support::{
    duplicate_pattern, is_wildcard, non_exhaustive, pattern_type_mismatch, variant_key,
};

impl Analyzer {
    pub(super) fn validate_case_guard(
        &mut self,
        guard: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let state = self.snapshot_binding_states();
        self.validate_guard_access(guard)?;
        self.visit_expression(guard)?;
        let found = self.expression_type_name(guard).unwrap_or_else(|| "unknown".to_owned());
        if found != "Bool" {
            self.restore_binding_states(&state);
            return Err(SemanticError {
                kind: SemanticErrorKind::GuardTypeMismatch { found },
                span,
            });
        }
        self.restore_binding_states(&state);
        Ok(())
    }

    fn validate_guard_access(&self, expression: &Expr) -> Result<(), SemanticError> {
        match expression {
            Expr::Identifier { name, span } => {
                let index = self.binding(name, *span)?;
                if self.model.bindings[index].ownership.is_live() {
                    Ok(())
                } else {
                    Err(SemanticError {
                        kind: SemanticErrorKind::InvalidGuardAccess { name: name.clone() },
                        span: *span,
                    })
                }
            }
            Expr::FieldAccess { object, field, span } => {
                self.validate_field_access(object, field, *span)?;
                self.ensure_field_access_readable(object, field, *span)
            }
            Expr::Grouping { expression, .. }
            | Expr::Borrow { expression, .. }
            | Expr::Try { expression, .. }
            | Expr::Unary { expression, .. }
            | Expr::Cast { expression, .. } => self.validate_guard_access(expression),
            Expr::Binary { left, right, .. } => self.validate_binary_guard_access(left, right),
            Expr::Call { callee, span, .. } | Expr::MethodCall { method: callee, span, .. } => {
                Err(self.invalid_guard_access(callee, *span))
            }
            Expr::Integer { .. }
            | Expr::BufferLiteral { .. }
            | Expr::FloatLiteral { .. }
            | Expr::StringLiteral { .. } => Ok(()),
            Expr::StructLit { name, span, .. } => Err(self.invalid_guard_access(name, *span)),
            Expr::Index { span, .. } => Err(self.invalid_guard_access("index", *span)),
            Expr::Case { span, .. } => Err(self.invalid_guard_access("case", *span)),
        }
    }

    fn validate_binary_guard_access(&self, left: &Expr, right: &Expr) -> Result<(), SemanticError> {
        self.validate_guard_access(left)?;
        self.validate_guard_access(right)
    }

    fn invalid_guard_access(&self, name: &str, span: SourceSpan) -> SemanticError {
        SemanticError {
            kind: SemanticErrorKind::InvalidGuardAccess { name: name.to_owned() },
            span,
        }
    }

    pub(super) fn validate_pattern_coverage(
        &self,
        subject: &Expr,
        branches: &[CaseBranch],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(enum_name) = self.expression_enum_type(subject) else {
            if branches.iter().any(|branch| is_wildcard(&branch.pattern) && branch.guard.is_none())
            {
                return Ok(());
            }
            return Err(non_exhaustive("primitive", vec!["_".to_owned()], span));
        };
        if branches.iter().any(|branch| is_wildcard(&branch.pattern) && branch.guard.is_none()) {
            return Ok(());
        }
        let covered = branches
            .iter()
            .filter(|branch| branch.guard.is_none())
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

    pub(super) fn validate_pattern(
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
                self.validate_payload_count(types.len(), bindings.len(), span)
            }
            (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => {
                self.validate_named_payload(fields.len(), &fields, patterns, span)
            }
            _ => Err(pattern_type_mismatch("matching payload", "different payload", span)),
        }
    }

    fn validate_payload_count(
        &self,
        expected: usize,
        found: usize,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if expected == found {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::EnumVariantArgumentCount {
                enum_name: "pattern".to_owned(),
                variant: "payload".to_owned(),
                expected,
                found,
            },
            span,
        })
    }

    fn validate_named_payload(
        &self,
        expected: usize,
        fields: &[crate::ast::EnumField],
        patterns: &[crate::ast::NamedPattern],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_payload_count(expected, patterns.len(), span)?;
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
}
