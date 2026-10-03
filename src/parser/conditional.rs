use crate::ast::{Expr, IfBranch, Stmt};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, Parser, expression_span};

impl Parser {
    pub(super) fn parse_if_expression(&mut self, start: SourceSpan) -> Result<Expr, ParseError> {
        let (condition, then_branch, else_branch, end) = self.parse_if_parts(false)?;
        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
            span: SourceSpan::new(start.start, end),
        })
    }

    pub(super) fn parse_if_statement(&mut self, start: SourceSpan) -> Result<Stmt, ParseError> {
        let (condition, then_branch, else_branch, end) = self.parse_if_parts(true)?;
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span: SourceSpan::new(start.start, end),
        })
    }

    fn parse_if_parts(
        &mut self,
        allow_statement_conditionals: bool,
    ) -> Result<(Expr, crate::ast::Block, Option<IfBranch>, usize), ParseError> {
        let condition = self.parse_expression()?;
        let then_branch = if allow_statement_conditionals {
            self.parse_block_with_statement_conditionals(true)?
        } else {
            self.parse_expression_block()?
        };
        let else_branch = self.parse_else_branch(allow_statement_conditionals)?;
        let end = else_branch.as_ref().map_or(then_branch.span.end, |branch| match branch {
            IfBranch::Block(block) => block.span.end,
            IfBranch::ElseIf(expression) => expression_span(expression).end,
        });
        Ok((condition, then_branch, else_branch, end))
    }

    fn parse_else_branch(
        &mut self,
        allow_statement_conditionals: bool,
    ) -> Result<Option<IfBranch>, ParseError> {
        if !self.match_simple(TokenKind::Else) {
            return Ok(None);
        }
        if self.match_simple(TokenKind::If) {
            return Ok(Some(IfBranch::ElseIf(Box::new(
                self.parse_if_expression(self.previous().span)?,
            ))));
        }
        let block = if allow_statement_conditionals {
            self.parse_block_with_statement_conditionals(true)?
        } else {
            self.parse_expression_block()?
        };
        Ok(Some(IfBranch::Block(block)))
    }
}
