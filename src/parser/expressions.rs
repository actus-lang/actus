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
        let expression = match token.kind {
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
            TokenKind::Identifier(name) => self.parse_identifier_expression(name, token.span),
            TokenKind::Integer(value) => Ok(Expr::Integer { value, span: token.span }),
            TokenKind::FloatLiteral(value) => Ok(Expr::FloatLiteral { value, span: token.span }),
            TokenKind::StringLiteral(value) => Ok(Expr::StringLiteral { value, span: token.span }),
            TokenKind::Ref => {
                let expression = self.parse_primary_expression()?;
                let span = SourceSpan::new(token.span.start, expression_span(&expression).end);
                Ok(Expr::Borrow { expression: Box::new(expression), span })
            }
            TokenKind::Case => self.parse_case_expression(),
            found => Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken { expected: "expression".to_owned(), found },
                span: token.span,
            }),
        }?;
        self.parse_field_access(expression)
    }

    fn parse_identifier_expression(
        &mut self,
        name: String,
        span: SourceSpan,
    ) -> Result<Expr, ParseError> {
        if name == "Buffer" && self.match_simple(TokenKind::LeftBracket) {
            let length = self.parse_expression()?;
            let end = self.expect_simple(TokenKind::RightBracket, "`]`")?.span.end;
            return Ok(Expr::BufferLiteral {
                length: Box::new(length),
                span: SourceSpan::new(span.start, end),
            });
        }
        let type_arguments = if self.match_simple(TokenKind::LeftBracket) {
            self.parse_type_arguments()?
        } else {
            Vec::new()
        };
        if self.match_simple(TokenKind::LeftParen) {
            if !type_arguments.is_empty() {
                return Err(self.error_at_current("a struct literal after type arguments"));
            }
            let arguments = self.parse_arguments()?;
            let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
            return Ok(Expr::Call {
                callee: name,
                arguments,
                span: SourceSpan::new(span.start, end),
            });
        }
        if !type_arguments.is_empty() && self.check_simple(&TokenKind::Dot) {
            let qualified_name = format_type_application(&name, &type_arguments);
            return Ok(Expr::Identifier { name: qualified_name, span });
        }
        if !self.case_subject && self.match_simple(TokenKind::LeftBrace) {
            return self.parse_struct_literal(name, type_arguments, span.start);
        }
        if !type_arguments.is_empty() {
            return Err(self.error_at_current("a struct literal after type arguments"));
        }
        Ok(Expr::Identifier { name, span })
    }

    fn parse_field_access(&mut self, mut expression: Expr) -> Result<Expr, ParseError> {
        while self.match_simple(TokenKind::Dot) {
            let field_token = self.take_identifier("field name after `.`")?;
            let field = identifier_text(&field_token.kind);
            if self.match_simple(TokenKind::LeftParen) {
                let start = expression_span(&expression).start;
                let arguments = self.parse_arguments()?;
                let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
                expression = Expr::MethodCall {
                    receiver: Box::new(expression),
                    method: field,
                    arguments,
                    span: SourceSpan::new(start, end),
                };
            } else {
                let span =
                    SourceSpan::new(expression_span(&expression).start, field_token.span.end);
                expression = Expr::FieldAccess { object: Box::new(expression), field, span };
            }
        }
        Ok(expression)
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
            let (role, role_span) = self.parse_argument_role()?;
            arguments.push(Argument {
                name,
                role,
                role_span,
                expression: self.parse_expression()?,
            });
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        Ok(arguments)
    }

    fn parse_argument_role(
        &mut self,
    ) -> Result<(Option<crate::ast::Role>, Option<SourceSpan>), ParseError> {
        let Some(token) = self.peek() else { return Ok((None, None)) };
        let Some(role) = role_from_token(&token.kind) else { return Ok((None, None)) };
        let span = token.span;
        self.advance_required("argument role")?;
        Ok((Some(role), Some(span)))
    }
}

fn role_from_token(kind: &TokenKind) -> Option<crate::ast::Role> {
    match kind {
        TokenKind::Erg => Some(crate::ast::Role::Erg),
        TokenKind::Abs => Some(crate::ast::Role::Abs),
        TokenKind::Dat => Some(crate::ast::Role::Dat),
        TokenKind::Ins => Some(crate::ast::Role::Ins),
        _ => None,
    }
}

fn format_type_application(name: &str, arguments: &[crate::ast::TypeName]) -> String {
    format!("{}[{}]", name, arguments.iter().map(format_type_name).collect::<Vec<_>>().join(","))
}

fn format_type_name(type_name: &crate::ast::TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format_type_application(&type_name.name, &type_name.arguments)
}
