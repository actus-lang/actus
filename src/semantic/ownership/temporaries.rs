//! Values that an expression produces and no binding receives (ADR-0125, bug
//! #128). A call result that a statement discards (`make();`) and a call that
//! a field or an element is read from (`pair().a.id`, `make()[0u32]`) are
//! temporaries: nothing owns them, so the analyzer records their release right
//! after the expression that last reads them, and code generation runs it.

use crate::ast::Expr;
use crate::lexer::SourceSpan;

use crate::semantic::analyzer::Analyzer;
use crate::semantic::calls::shapes::expression_span;
use crate::semantic::cleanup::TemporaryRelease;

impl Analyzer {
    /// Records the release of the value of the expression statement
    /// `expression` at `statement_span` when the statement discards it: a
    /// call written with its `;`. The tail of a block without `;` is the
    /// block's value, which its receiver owns.
    pub(in crate::semantic) fn release_discarded_value(
        &mut self,
        expression: &Expr,
        statement_span: SourceSpan,
    ) {
        let Expr::Call { span, .. } = expression else { return };
        if statement_span.end == span.end {
            return;
        }
        self.record_temporary(expression, *span, *span);
    }

    /// Takes back the release that `release_discarded_value` recorded for
    /// `expression` when it turns out to be the value of its block: the tail
    /// of an `if` branch or a `case` arm, which its receiver owns.
    pub(in crate::semantic) fn keep_tail_value(&mut self, expression: &Expr) {
        let Expr::Call { span, .. } = expression else { return };
        self.model
            .temporary_releases
            .retain(|temporary| temporary.span != *span || temporary.release_after != *span);
    }

    /// Records the release of the call that the field or element read
    /// `expression` starts from, once the read is evaluated. Only the
    /// outermost read of a path releases it: the read of a value that would
    /// move (a struct, an enum, a resource) is an inner step of a longer
    /// path, or a move out of a temporary, which is rejected elsewhere.
    pub(in crate::semantic) fn release_read_temporary(&mut self, expression: &Expr) {
        let Some(call) = temporary_root(expression) else { return };
        if self.is_move_only_aggregate(expression) || self.resolved_type_name(expression).is_none()
        {
            return;
        }
        let Expr::Call { span, .. } = call else { return };
        self.record_temporary(call, *span, expression_span(expression));
    }

    /// Records that the temporary `call` at `span`, when its type holds a
    /// resource, is released after the expression at `release_after`. An
    /// expression the analyzer visits twice is recorded once.
    fn record_temporary(&mut self, call: &Expr, span: SourceSpan, release_after: SourceSpan) {
        let Some(type_name) = self.resolved_type_name(call) else { return };
        if !self.type_holds_resource(&type_name)
            || self.model.temporary_releases.iter().any(|temporary| temporary.span == span)
        {
            return;
        }
        self.model.temporary_releases.push(TemporaryRelease { span, release_after, type_name });
    }
}

/// The call a field or element path starts from (`pair()` in `pair().a.id`),
/// or `None` when the path starts at a binding or anything else.
fn temporary_root(expression: &Expr) -> Option<&Expr> {
    let base = match expression {
        Expr::FieldAccess { object, .. } => object.as_ref(),
        Expr::Index { target, .. } => target.as_ref(),
        _ => return None,
    };
    match base {
        Expr::Call { .. } => Some(base),
        Expr::FieldAccess { .. } | Expr::Index { .. } => temporary_root(base),
        _ => None,
    }
}
