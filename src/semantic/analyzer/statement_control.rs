use crate::ast::Block;
use crate::lexer::SourceSpan;

use super::super::errors::SemanticError;
use super::Analyzer;

impl Analyzer {
    pub(super) fn visit_loop(&mut self, block: &Block) -> Result<(), SemanticError> {
        self.enter_scope(block.span);
        self.loop_boundaries.push(self.scopes.len() - 1);
        self.visit_block(block)?;
        self.loop_boundaries.pop();
        self.leave_scope();
        Ok(())
    }

    pub(super) fn visit_loop_control(
        &mut self,
        is_break: bool,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let kind = if is_break {
            super::super::cleanup::LoopExitKind::Break
        } else {
            super::super::cleanup::LoopExitKind::Continue
        };
        self.plan_loop_unwind(kind, if is_break { "break" } else { "continue" }, span)
    }

    pub(super) fn visit_nested_block(&mut self, block: &Block) -> Result<(), SemanticError> {
        self.enter_scope(block.span);
        self.visit_block(block)?;
        self.leave_scope();
        Ok(())
    }
}
