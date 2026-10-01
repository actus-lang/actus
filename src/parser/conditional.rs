use crate::ast::{Expr, IfBranch};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, Parser, expression_span};

impl Parser {
    pub(super) fn parse_if_expression(&mut self, start: SourceSpan) -> Result<Expr, ParseError> {
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;
        let else_branch = self.parse_else_branch()?;
        let end = else_branch.as_ref().map_or(then_branch.span.end, |branch| match branch {
            IfBranch::Block(block) => block.span.end,
            IfBranch::ElseIf(expression) => expression_span(expression).end,
        });
        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
            span: SourceSpan::new(start.start, end),
        })
    }

    fn parse_else_branch(&mut self) -> Result<Option<IfBranch>, ParseError> {
        if !self.match_simple(TokenKind::Else) {
            return Ok(None);
        }
        if self.match_simple(TokenKind::If) {
            return Ok(Some(IfBranch::ElseIf(Box::new(
                self.parse_if_expression(self.previous().span)?,
            ))));
        }
        Ok(Some(IfBranch::Block(self.parse_block()?)))
    }
}
