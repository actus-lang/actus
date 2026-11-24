use crate::ast::{Expr, PrimitiveType, Stmt, primitive_type};
use crate::lexer::SourceSpan;

use super::VerbSignature;
use super::analyzer::{Analyzer, expression_span};
use super::errors::{SemanticError, SemanticErrorKind};

fn returns_nothing(signature: &VerbSignature) -> bool {
    signature
        .return_type_name
        .as_ref()
        .is_none_or(|return_type| primitive_type(&return_type.name) == Some(PrimitiveType::Void))
}

impl Analyzer {
    /// Rejects a call to a verb that returns nothing unless its value is dropped
    /// by a statement (E1141).
    pub(in crate::semantic) fn reject_void_call_value(
        &self,
        callee: &str,
        signature: &VerbSignature,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if returns_nothing(signature) && !self.call_value_dropped {
            return Err(SemanticError {
                kind: SemanticErrorKind::VoidValue { callee: callee.to_owned() },
                span,
            });
        }
        Ok(())
    }

    /// The span and verb name of an expression that is a direct call to a verb
    /// returning nothing, once the call has been resolved by its name.
    fn void_call(&self, expression: &Expr) -> Option<(SourceSpan, String)> {
        let (Expr::Call { callee: name, .. } | Expr::MethodCall { method: name, .. }) = expression
        else {
            return None;
        };
        let signature = self
            .call_signature_name(name)
            .and_then(|lookup| self.signatures.get(&lookup))
            .or_else(|| self.signatures.get(name.as_str()))?;
        returns_nothing(signature).then(|| (expression_span(expression), name.clone()))
    }

    /// Rejects a value that is a direct call to a verb returning nothing
    /// before type checks that would report it as an unknown type.
    pub(in crate::semantic) fn reject_void_value(
        &self,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        match self.void_call(expression) {
            Some((span, callee)) => {
                Err(SemanticError { kind: SemanticErrorKind::VoidValue { callee }, span })
            }
            None => Ok(()),
        }
    }

    /// Rejects the unterminated last statement of a block that supplies the
    /// value of an `if` or `case` expression when it calls a verb returning
    /// nothing.
    pub(in crate::semantic) fn reject_void_block_value(
        &self,
        block: &crate::ast::Block,
    ) -> Result<(), SemanticError> {
        match block.statements.last() {
            Some(Stmt::Expression { expression, span })
                if span.end == expression_span(expression).end =>
            {
                self.reject_void_value(expression)
            }
            _ => Ok(()),
        }
    }
}
