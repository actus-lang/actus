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
        Ok(())
    }
}
