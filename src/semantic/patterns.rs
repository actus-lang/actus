use std::collections::HashSet;

use crate::ast::{
    CaseBody, CaseBranch, EnumPayload, Expr, LiteralPattern, Pattern, PatternBinding, Role,
    VariantPayload,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::pattern_support::{
    duplicate_pattern, is_wildcard, non_exhaustive, pattern_name, pattern_span,
    pattern_type_mismatch, unreachable_pattern, variant_key,
};
use super::state::{AccessState, OwnershipState};

impl Analyzer {
    pub(super) fn validate_case_patterns(
        &mut self,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[CaseBranch],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_pattern_coverage(subject, branches, span)?;
        let subject_type =
            self.expression_type_name(subject).unwrap_or_else(|| "unknown".to_owned());
        self.enter_scope(span);
        match mode {
            crate::ast::CaseMode::Abs => self.borrow_case_subject(subject, span)?,
            crate::ast::CaseMode::Dat => self.consume_case_subject(subject, span)?,
        }
        let branch_state = self.snapshot_binding_states();
        let branch_count = branch_state.len();
        let mut branch_results = Vec::new();
        let mut seen = HashSet::new();
        let mut wildcard_seen = false;
        for branch in branches {
            self.restore_binding_states(&branch_state);
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
            self.bind_pattern_variables(&branch.pattern, mode)?;
            if mode == crate::ast::CaseMode::Dat {
                self.register_unbound_payload_cleanup(subject, &branch.pattern)?;
            }
            if let Some(guard) = &branch.guard {
                self.validate_case_guard(guard, branch.span)?;
            }
            self.visit_case_body(&branch.body)?;
            self.leave_scope();
            branch_results.push(self.snapshot_binding_prefix(branch_count));
            self.restore_binding_states(&branch_state);
        }
        self.validate_branch_join(&branch_results, span)?;
        if let Some(joined_state) = branch_results.first() {
            self.restore_binding_states(joined_state);
        }
        self.leave_scope();
        Ok(())
    }

    fn snapshot_binding_states(&self) -> Vec<(OwnershipState, AccessState)> {
        self.model
            .bindings
            .iter()
            .map(|binding| (binding.ownership.clone(), binding.access.clone()))
            .collect()
    }

    fn restore_binding_states(&mut self, snapshot: &[(OwnershipState, AccessState)]) {
        for (binding, (ownership, access)) in self.model.bindings.iter_mut().zip(snapshot.iter()) {
            binding.ownership = ownership.clone();
            binding.access = access.clone();
        }
    }

    fn snapshot_binding_prefix(&self, count: usize) -> Vec<(OwnershipState, AccessState)> {
        self.snapshot_binding_states().into_iter().take(count).collect()
    }

    fn validate_branch_join(
        &self,
        branch_results: &[Vec<(OwnershipState, AccessState)>],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expected_states) = branch_results.first() else { return Ok(()) };
        for states in branch_results.iter().skip(1) {
            for (index, (expected, found)) in expected_states.iter().zip(states).enumerate() {
                if expected != found {
                    let name = self.model.bindings[index].name.clone();
                    return Err(SemanticError {
                        kind: SemanticErrorKind::BranchStateMismatch {
                            name,
                            expected: format!("{expected:?}"),
                            found: format!("{found:?}"),
                        },
                        span,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_case_guard(&mut self, guard: &Expr, span: SourceSpan) -> Result<(), SemanticError> {
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
            | Expr::Unary { expression, .. } => self.validate_guard_access(expression),
            Expr::Binary { left, right, .. } => {
                self.validate_guard_access(left)?;
                self.validate_guard_access(right)
            }
            Expr::Call { callee, span, .. } | Expr::MethodCall { method: callee, span, .. } => {
                Err(SemanticError {
                    kind: SemanticErrorKind::InvalidGuardAccess { name: callee.clone() },
                    span: *span,
                })
            }
            Expr::Integer { .. } | Expr::FloatLiteral { .. } | Expr::StringLiteral { .. } => Ok(()),
            Expr::StructLit { name, span, .. } => Err(SemanticError {
                kind: SemanticErrorKind::InvalidGuardAccess { name: name.clone() },
                span: *span,
            }),
            Expr::Case { span, .. } => Err(SemanticError {
                kind: SemanticErrorKind::InvalidGuardAccess { name: "case".to_owned() },
                span: *span,
            }),
        }
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

    fn bind_pattern_variables(
        &mut self,
        pattern: &Pattern,
        mode: crate::ast::CaseMode,
    ) -> Result<(), SemanticError> {
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
                            self.bind_pattern_binding(binding, &ty.name, mode)?;
                        }
                    }
                    (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => {
                        for pattern in patterns {
                            let field =
                                fields.iter().find(|field| field.name == pattern.name).unwrap();
                            self.bind_pattern_binding(&pattern.binding, &field.ty.name, mode)?;
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
        mode: crate::ast::CaseMode,
    ) -> Result<(), SemanticError> {
        if binding.name == "_" {
            return Ok(());
        }
        let is_generic_payload = self.enum_types.values().any(|definition| {
            definition.generic_parameters.iter().any(|parameter| parameter.name == type_name)
        });
        if !self.is_generic_parameter(type_name) && !is_generic_payload {
            self.validate_type_name(type_name, binding.span)?;
        }
        self.bind(
            match mode {
                crate::ast::CaseMode::Abs => Role::Abs,
                crate::ast::CaseMode::Dat => Role::Dat,
            },
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
