use crate::ast::Block;
use crate::lexer::SourceSpan;

use super::super::errors::SemanticError;
use super::Analyzer;

impl Analyzer {
    pub(in crate::semantic::analyzer) fn visit_loop(
        &mut self,
        block: &Block,
    ) -> Result<(), SemanticError> {
        if self.current_realtime {
            return Err(SemanticError {
                kind: super::super::errors::SemanticErrorKind::RealtimeUnboundedLoop,
                span: block.span,
            });
        }
        self.enter_scope(block.span);
        self.loop_boundaries.push(self.scopes.len() - 1);
        self.enter_loop_ownership();
        let result = self.visit_block(block);
        let result = self.leave_loop_ownership(block, false, result);
        self.loop_boundaries.pop();
        self.leave_scope();
        result
    }

    pub(in crate::semantic::analyzer) fn visit_loop_control(
        &mut self,
        is_break: bool,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if is_break {
            self.record_loop_break();
        } else {
            self.reject_moves_into_next_iteration()?;
        }
        let kind = if is_break {
            super::super::cleanup::LoopExitKind::Break
        } else {
            super::super::cleanup::LoopExitKind::Continue
        };
        self.plan_loop_unwind(kind, if is_break { "break" } else { "continue" }, span)
    }

    pub(in crate::semantic::analyzer) fn visit_nested_block(
        &mut self,
        block: &Block,
    ) -> Result<(), SemanticError> {
        self.enter_scope(block.span);
        self.visit_block(block)?;
        self.leave_scope();
        Ok(())
    }
}
