use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::VerbSignature;

impl Analyzer {
    pub(super) fn validate_effects(
        &self,
        callee: &str,
        signature: &VerbSignature,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        for effect in &signature.effects {
            if self.current_realtime {
                return Err(SemanticError {
                    kind: SemanticErrorKind::RealtimeEffect {
                        callee: callee.to_owned(),
                        effect: *effect,
                    },
                    span,
                });
            }
            if self.current_deterministic && effect.is_nondeterministic() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::NondeterministicCall {
                        callee: callee.to_owned(),
                        effect: *effect,
                    },
                    span,
                });
            }
            if !self.current_effects.contains(effect) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UndeclaredEffect {
                        callee: callee.to_owned(),
                        effect: *effect,
                    },
                    span,
                });
            }
        }
        self.validate_bounded_call(callee, signature, span)?;
        if self.current_realtime
            && signature.return_type_name.as_ref().is_some_and(type_contains_float)
        {
            return Err(SemanticError { kind: SemanticErrorKind::RealtimeFloatingPoint, span });
        }
        if self.current_realtime && self.current_verb_name.as_deref() == Some(callee) {
            return Err(SemanticError {
                kind: SemanticErrorKind::RealtimeUnboundedRecursion,
                span,
            });
        }
        Ok(())
    }

    fn validate_bounded_call(
        &self,
        callee: &str,
        signature: &VerbSignature,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.current_bounded.is_some() && signature.bounded.is_none() {
            return Err(bounded_error(
                "max_iterations",
                0,
                &format!("callee `{callee}` must declare a bounded contract"),
                span,
            ));
        }
        if self.current_bounded.is_some() && self.current_verb_name.as_deref() == Some(callee) {
            return Err(bounded_error("max_call_depth", 0, "recursive call path", span));
        }
        Ok(())
    }
}

fn bounded_error(dimension: &str, limit: u64, required: &str, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::BoundedBudgetExceeded {
            dimension: dimension.to_owned(),
            limit,
            required: required.to_owned(),
        },
        span,
    }
}

fn type_contains_float(type_name: &crate::ast::TypeName) -> bool {
    matches!(type_name.name.as_str(), "f32" | "f64")
        || type_name.arguments.iter().any(type_contains_float)
}
