use crate::ast::{Argument, BinaryOp, Expr, UnaryOp};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, expression_span, identifier_text};

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, minimum_precedence: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary_expression()?;
        while let Some((operator, precedence)) = self.binary_operator() {
            if precedence < minimum_precedence {
                break;
            }
            self.advance_required("binary operator")?;
            let right = self.parse_binary_expression(precedence + 1)?;
            let span = SourceSpan::new(expression_span(&left).start, expression_span(&right).end);
            left = Expr::Binary { left: Box::new(left), operator, right: Box::new(right), span };
        }
        Ok(left)
    }

    fn parse_primary_expression(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance_required("expression")?;
        match token.kind {
            TokenKind::Minus => {
                let expression = self.parse_primary_expression()?;
                let span = SourceSpan::new(token.span.start, expression_span(&expression).end);
                Ok(Expr::Unary {
                    operator: UnaryOp::Negate,
                    expression: Box::new(expression),
                    span,
                })
            }
            TokenKind::LeftParen => {
                let expression = self.parse_expression()?;
                let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
                Ok(Expr::Grouping {
                    expression: Box::new(expression),
                    span: SourceSpan::new(token.span.start, end),
                })
            }
            TokenKind::Identifier(name) => {
                if self.match_simple(TokenKind::LeftParen) {
                    let arguments = self.parse_arguments()?;
                    let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
                    Ok(Expr::Call {
                        callee: name,
                        arguments,
                        span: SourceSpan::new(token.span.start, end),
                    })
                } else {
                    Ok(Expr::Identifier { name, span: token.span })
                }
            }
            TokenKind::Integer(value) => Ok(Expr::Integer { value, span: token.span }),
            TokenKind::StringLiteral(value) => Ok(Expr::StringLiteral { value, span: token.span }),
            TokenKind::Ref => {
                let expression = self.parse_primary_expression()?;
                let span = SourceSpan::new(token.span.start, expression_span(&expression).end);
                Ok(Expr::Borrow { expression: Box::new(expression), span })
            }
            found => Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken { expected: "expression".to_owned(), found },
                span: token.span,
            }),
        }
    }

    fn binary_operator(&self) -> Option<(BinaryOp, u8)> {
        let operator = match self.peek()?.kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Subtract,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Slash => BinaryOp::Divide,
            _ => return None,
        };
        let precedence = match operator {
            BinaryOp::Add | BinaryOp::Subtract => 1,
            BinaryOp::Multiply | BinaryOp::Divide => 2,
        };
        Some((operator, precedence))
    }

    fn parse_arguments(&mut self) -> Result<Vec<Argument>, ParseError> {
        let mut arguments = Vec::new();
        if self.check_simple(&TokenKind::RightParen) {
            return Ok(arguments);
        }
        loop {
            let name = if self.check_identifier() && self.peek_next_is(&TokenKind::Colon) {
                let name = identifier_text(&self.advance_required("argument name")?.kind);
                self.expect_simple(TokenKind::Colon, "`:`")?;
                Some(name)
            } else {
                None
            };
            arguments.push(Argument { name, expression: self.parse_expression()? });
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        Ok(arguments)
    }
}
