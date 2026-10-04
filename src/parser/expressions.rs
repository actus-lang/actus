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
            TokenKind::Minus | TokenKind::Bang | TokenKind::Tilde => {
                self.parse_unary_prefix(token.span, unary_operator(&token.kind))
            }
            TokenKind::LeftParen => self.parse_grouped_prefix(token.span),
            TokenKind::Identifier(name) => self.parse_identifier_expression(name, token.span),
            TokenKind::Integer { value, suffix } => {
                Ok(Expr::Integer { value, suffix, span: token.span })
            }
            TokenKind::True => Ok(Expr::BoolLiteral { value: true, span: token.span }),
            TokenKind::False => Ok(Expr::BoolLiteral { value: false, span: token.span }),
            TokenKind::FloatLiteral { value, suffix } => {
                Ok(Expr::FloatLiteral { value, suffix, span: token.span })
            }
            TokenKind::StringLiteral(value) => Ok(Expr::StringLiteral { value, span: token.span }),
            TokenKind::Ref => self.parse_borrow_prefix(token.span),
            TokenKind::Case => self.parse_case_expression(),
            TokenKind::If => self.parse_if_expression(token.span),
            found => Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken { expected: "expression".to_owned(), found },
                span: token.span,
            }),
        }?;
        self.parse_field_access(expression)
    }

    fn parse_unary_prefix(
        &mut self,
        start: SourceSpan,
        operator: UnaryOp,
    ) -> Result<Expr, ParseError> {
        let expression = self.parse_primary_expression()?;
        let span = SourceSpan::new(start.start, expression_span(&expression).end);
        Ok(Expr::Unary { operator, expression: Box::new(expression), span })
    }

    fn parse_grouped_prefix(&mut self, start: SourceSpan) -> Result<Expr, ParseError> {
        let expression = self.parse_expression()?;
        let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
        Ok(Expr::Grouping {
            expression: Box::new(expression),
            span: SourceSpan::new(start.start, end),
        })
    }

    fn parse_borrow_prefix(&mut self, start: SourceSpan) -> Result<Expr, ParseError> {
        let expression = self.parse_primary_expression()?;
        let span = SourceSpan::new(start.start, expression_span(&expression).end);
        Ok(Expr::Borrow { expression: Box::new(expression), span })
    }

    fn parse_identifier_expression(
        &mut self,
        name: String,
        span: SourceSpan,
    ) -> Result<Expr, ParseError> {
        if let Some(buffer) = self.parse_buffer_literal(&name, span)? {
            return Ok(buffer);
        }
        let type_arguments = if self.should_parse_type_arguments() {
            self.expect_simple(TokenKind::LeftBracket, "`[`")?;
            self.parse_type_arguments()?
        } else {
            Vec::new()
        };
        if let Some(call) = self.parse_identifier_call(&name, &type_arguments, span)? {
            return Ok(call);
        }
        self.parse_identifier_suffix(name, type_arguments, span)
    }

    fn should_parse_type_arguments(&self) -> bool {
        if !self.check_simple(&TokenKind::LeftBracket) {
            return false;
        }
        let mut depth = 0usize;
        let mut closing = None;
        for (offset, token) in self.tokens[self.cursor..].iter().enumerate() {
            match token.kind {
                TokenKind::LeftBracket => depth += 1,
                TokenKind::RightBracket => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        closing = Some(self.cursor + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(closing) = closing else { return false };
        let Some(next) = self.tokens.get(closing + 1) else { return false };
        if matches!(next.kind, TokenKind::LeftParen | TokenKind::LeftBrace) {
            return true;
        }
        matches!(next.kind, TokenKind::Dot) && self.tokens.get(self.cursor + 1).is_some_and(|token| {
            matches!(&token.kind, TokenKind::Identifier(name) if name.chars().next().is_some_and(char::is_uppercase))
                || matches!(
                    token.kind,
                    TokenKind::IntType { .. }
                        | TokenKind::FloatType { .. }
                        | TokenKind::VoidType
                        | TokenKind::Abs
                        | TokenKind::Ins
                )
        })
    }

    fn parse_buffer_literal(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<Option<Expr>, ParseError> {
        if name != "Buffer" || !self.match_simple(TokenKind::LeftBracket) {
            return Ok(None);
        }
        let length = self.parse_expression()?;
        let end = self.expect_simple(TokenKind::RightBracket, "`]`")?.span.end;
        Ok(Some(Expr::BufferLiteral {
            length: Box::new(length),
            span: SourceSpan::new(span.start, end),
        }))
    }

    fn parse_identifier_call(
        &mut self,
        name: &str,
        type_arguments: &[crate::ast::TypeName],
        span: SourceSpan,
    ) -> Result<Option<Expr>, ParseError> {
        if self.match_simple(TokenKind::LeftParen) {
            let arguments = self.parse_arguments()?;
            let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
            let callee = if type_arguments.is_empty() {
                name.to_owned()
            } else {
                format_type_application(name, type_arguments)
            };
            return Ok(Some(Expr::Call {
                callee,
                arguments,
                span: SourceSpan::new(span.start, end),
            }));
        }
        Ok(None)
    }

    fn parse_identifier_suffix(
        &mut self,
        name: String,
        type_arguments: Vec<crate::ast::TypeName>,
        span: SourceSpan,
    ) -> Result<Expr, ParseError> {
        if !type_arguments.is_empty() && self.check_simple(&TokenKind::Dot) {
            let qualified_name = format_type_application(&name, &type_arguments);
            return Ok(Expr::Identifier { name: qualified_name, span });
        }
        let is_type_name = name.chars().next().is_some_and(char::is_uppercase);
        if !self.case_subject
            && (is_type_name || !type_arguments.is_empty())
            && self.looks_like_struct_literal()
        {
            self.expect_simple(TokenKind::LeftBrace, "`{`")?;
            return self.parse_struct_literal(name, type_arguments, span.start);
        }
        if !type_arguments.is_empty() {
            return Err(self.error_at_current("a struct literal after type arguments"));
        }
        Ok(Expr::Identifier { name, span })
    }

    fn looks_like_struct_literal(&self) -> bool {
        if !self.check_simple(&TokenKind::LeftBrace) {
            return false;
        }
        let Some(first) = self.tokens.get(self.cursor + 1) else { return false };
        if matches!(first.kind, TokenKind::RightBrace) {
            return true;
        }
        matches!(
            (&first.kind, self.tokens.get(self.cursor + 2).map(|token| &token.kind)),
            (TokenKind::Identifier(_), Some(TokenKind::Colon))
        )
    }

    fn parse_field_access(&mut self, mut expression: Expr) -> Result<Expr, ParseError> {
        loop {
            if self.match_simple(TokenKind::LeftBracket) {
                let index = self.parse_expression()?;
                let end = self.expect_simple(TokenKind::RightBracket, "`]`")?.span.end;
                let span = SourceSpan::new(expression_span(&expression).start, end);
                expression =
                    Expr::Index { target: Box::new(expression), index: Box::new(index), span };
                continue;
            }
            if self.match_simple(TokenKind::Question) {
                let span =
                    SourceSpan::new(expression_span(&expression).start, self.previous().span.end);
                expression = Expr::Try { expression: Box::new(expression), span };
                continue;
            }
            if self.match_simple(TokenKind::As) {
                let target = self.parse_type_name()?;
                let span = SourceSpan::new(expression_span(&expression).start, target.span.end);
                expression = Expr::Cast { expression: Box::new(expression), target, span };
                continue;
            }
            if !self.match_simple(TokenKind::Dot) {
                break;
            }
            let field_token = self.take_identifier("field name after `.`")?;
            expression = self.parse_member_access(expression, field_token)?;
        }
        Ok(expression)
    }

    fn parse_member_access(
        &mut self,
        expression: Expr,
        field_token: crate::lexer::Token,
    ) -> Result<Expr, ParseError> {
        let field = identifier_text(&field_token.kind);
        if self.match_simple(TokenKind::LeftParen) {
            let start = expression_span(&expression).start;
            let arguments = self.parse_arguments()?;
            let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
            return Ok(Expr::MethodCall {
                receiver: Box::new(expression),
                method: field,
                arguments,
                span: SourceSpan::new(start, end),
            });
        }
        let span = SourceSpan::new(expression_span(&expression).start, field_token.span.end);
        Ok(Expr::FieldAccess { object: Box::new(expression), field, span })
    }

    fn binary_operator(&self) -> Option<(BinaryOp, u8)> {
        let operator = match self.peek()?.kind {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Subtract,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Slash => BinaryOp::Divide,
            TokenKind::Percent => BinaryOp::Remainder,
            TokenKind::ShiftLeft => BinaryOp::ShiftLeft,
            TokenKind::ShiftRight => BinaryOp::ShiftRight,
            TokenKind::Ampersand => BinaryOp::BitwiseAnd,
            TokenKind::Caret => BinaryOp::BitwiseXor,
            TokenKind::Pipe => BinaryOp::BitwiseOr,
            TokenKind::LessThan => BinaryOp::LessThan,
            TokenKind::LessEquals => BinaryOp::LessEquals,
            TokenKind::GreaterThan => BinaryOp::GreaterThan,
            TokenKind::GreaterEquals => BinaryOp::GreaterEquals,
            TokenKind::DoubleEquals => BinaryOp::Equals,
            TokenKind::BangEquals => BinaryOp::NotEquals,
            TokenKind::AndAnd => BinaryOp::LogicalAnd,
            TokenKind::OrOr => BinaryOp::LogicalOr,
            _ => return None,
        };
        let precedence = match operator {
            BinaryOp::LogicalOr => 0,
            BinaryOp::LogicalAnd => 1,
            BinaryOp::Equals | BinaryOp::NotEquals => 2,
            BinaryOp::LessThan
            | BinaryOp::LessEquals
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterEquals => 3,
            BinaryOp::BitwiseOr => 4,
            BinaryOp::BitwiseXor => 5,
            BinaryOp::BitwiseAnd => 6,
            BinaryOp::ShiftLeft | BinaryOp::ShiftRight => 7,
            BinaryOp::Add | BinaryOp::Subtract => 8,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 9,
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

fn unary_operator(kind: &TokenKind) -> UnaryOp {
    match kind {
        TokenKind::Bang => UnaryOp::LogicalNot,
        TokenKind::Tilde => UnaryOp::BitwiseNot,
        TokenKind::Minus => UnaryOp::Negate,
        _ => unreachable!("unary_operator called for a non-unary token"),
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
    let role = type_name
        .reference_role
        .as_ref()
        .map(|role| match role {
            crate::ast::Role::Abs => "abs ",
            crate::ast::Role::Ins => "ins ",
            crate::ast::Role::Erg => "erg ",
            crate::ast::Role::Dat => "dat ",
        })
        .unwrap_or("");
    if type_name.arguments.is_empty() {
        return format!("{role}{}", type_name.name);
    }
    format!("{role}{}", format_type_application(&type_name.name, &type_name.arguments))
}
