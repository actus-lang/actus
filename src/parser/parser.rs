use crate::ast::{Block, Expr, Param, Program, Role, Stmt, TopLevelDecl, TypeName, VerbDecl};
use crate::lexer::{SourceSpan, Token, TokenKind};

mod expressions;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedToken { expected: String, found: TokenKind },
    UnexpectedEndOfInput { expected: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseErrorCode {
    UnexpectedToken,
    UnexpectedEndOfInput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub kind: ParseErrorKind,
    pub span: SourceSpan,
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn parse(mut self) -> Result<Program, ParseError> {
        let mut declarations = Vec::new();

        while !self.at_end() {
            declarations.push(self.parse_top_level_decl()?);
        }

        Ok(Program { declarations })
    }

    fn parse_top_level_decl(&mut self) -> Result<TopLevelDecl, ParseError> {
        let declaration = self.parse_verb()?;
        Ok(TopLevelDecl::Verb(declaration))
    }

    fn parse_verb(&mut self) -> Result<VerbDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Verb, "`verb`")?.span.start;
        let name_token = self.take_identifier("verb name")?;
        let name = identifier_text(&name_token.kind);

        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RightParen, "`)`")?;

        let return_type =
            if self.match_simple(TokenKind::Arrow) { Some(self.parse_type_name()?) } else { None };

        let body = self.parse_block()?;
        let span = SourceSpan::new(start, body.span.end);

        Ok(VerbDecl { name, params, return_type, body, span })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();

        if self.check_simple(&TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            params.push(self.parse_param()?);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let role_token = self.advance_required("parameter role")?;
        let role = role_from_token(&role_token.kind).ok_or_else(|| ParseError {
            code: ParseErrorCode::UnexpectedToken,
            kind: ParseErrorKind::UnexpectedToken {
                expected: "parameter role (`erg`, `abs`, or `dat`)".to_owned(),
                found: role_token.kind.clone(),
            },
            span: role_token.span,
        })?;

        let name_token = self.take_identifier("parameter name")?;
        let name = identifier_text(&name_token.kind);
        self.expect_simple(TokenKind::Colon, "`:`")?;
        let ty = self.parse_type_name()?;

        Ok(Param { role, name, span: SourceSpan::new(role_token.span.start, ty.span.end), ty })
    }

    fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        let token = self.take_identifier("type name")?;
        Ok(TypeName { name: identifier_text(&token.kind), span: token.span })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect_simple(TokenKind::LeftBrace, "`{`")?.span.start;
        let mut statements = Vec::new();

        while !self.check_simple(&TokenKind::RightBrace) {
            if self.at_end() {
                return Err(self.error_at_current("`}`"));
            }
            statements.push(self.parse_statement()?);
        }

        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(Block { statements, span: SourceSpan::new(start, end) })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.check_simple(&TokenKind::LeftBrace) {
            return Ok(Stmt::Block(self.parse_block()?));
        }

        if self.match_simple(TokenKind::Loop) {
            return Ok(Stmt::Loop(self.parse_block()?));
        }

        if self.check_role(&TokenKind::Erg) || self.check_role(&TokenKind::Abs) {
            return self.parse_owner_declaration();
        }

        if self.match_simple(TokenKind::Return) {
            return self.parse_return_statement();
        }

        if self.match_simple(TokenKind::Break) {
            return self.parse_loop_control_statement(true);
        }

        if self.match_simple(TokenKind::Continue) {
            return self.parse_loop_control_statement(false);
        }

        if self.match_simple(TokenKind::Drop) {
            return self.parse_drop_statement();
        }

        let expression = self.parse_expression()?;
        if let Expr::Identifier { name, span } = &expression
            && self.match_simple(TokenKind::Equals)
        {
            let value = self.parse_expression()?;
            let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
            return Ok(Stmt::Assignment {
                name: name.clone(),
                value,
                span: SourceSpan::new(span.start, end),
            });
        }

        let start = expression_span(&expression).start;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(Stmt::Expression { expression, span: SourceSpan::new(start, end) })
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.previous().span.start;
        let value = (!self.check_simple(&TokenKind::Semicolon))
            .then(|| self.parse_expression())
            .transpose()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(Stmt::Return { value, span: SourceSpan::new(start, end) })
    }

    fn parse_loop_control_statement(&mut self, is_break: bool) -> Result<Stmt, ParseError> {
        let start = self.previous().span.start;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        let span = SourceSpan::new(start, end);
        Ok(if is_break { Stmt::Break { span } } else { Stmt::Continue { span } })
    }

    fn parse_drop_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.previous().span.start;
        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let name = identifier_text(&self.take_identifier("binding name")?.kind);
        self.expect_simple(TokenKind::RightParen, "`)`")?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(Stmt::Drop { name, span: SourceSpan::new(start, end) })
    }

    fn parse_owner_declaration(&mut self) -> Result<Stmt, ParseError> {
        let role_token = self.advance_required("binding role")?;
        let role = role_from_token(&role_token.kind).ok_or_else(|| ParseError {
            code: ParseErrorCode::UnexpectedToken,
            kind: ParseErrorKind::UnexpectedToken {
                expected: "`erg` or `abs`".to_owned(),
                found: role_token.kind.clone(),
            },
            span: role_token.span,
        })?;
        let name_token = self.take_identifier("binding name")?;
        let name = identifier_text(&name_token.kind);
        let ty = if self.match_simple(TokenKind::Colon) {
            Some(identifier_text(&self.take_identifier("binding type")?.kind))
        } else {
            None
        };
        self.expect_simple(TokenKind::Equals, "`=`")?;
        let initializer = if role == Role::Abs {
            self.expect_simple(TokenKind::Ref, "`ref`")?;
            let expression = self.parse_expression()?;
            let span = SourceSpan::new(role_token.span.start, expression_span(&expression).end);
            Expr::Borrow { expression: Box::new(expression), span }
        } else {
            self.parse_expression()?
        };
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;

        Ok(Stmt::OwnerDecl {
            role,
            name,
            ty,
            initializer,
            span: SourceSpan::new(role_token.span.start, end),
        })
    }

    fn take_identifier(&mut self, expected: &str) -> Result<Token, ParseError> {
        let token = self.advance_required(expected)?;
        if matches!(token.kind, TokenKind::Identifier(_)) {
            Ok(token)
        } else {
            Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: expected.to_owned(),
                    found: token.kind,
                },
                span: token.span,
            })
        }
    }

    fn expect_keyword(&mut self, expected: TokenKind, label: &str) -> Result<Token, ParseError> {
        self.expect_simple(expected, label)
    }

    fn expect_simple(&mut self, expected: TokenKind, label: &str) -> Result<Token, ParseError> {
        if self.check_simple(&expected) {
            Ok(self.advance_required(label)?)
        } else {
            Err(self.error_at_current(label))
        }
    }

    fn match_simple(&mut self, expected: TokenKind) -> bool {
        if self.check_simple(&expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn check_simple(&self, expected: &TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == *expected)
    }

    fn check_role(&self, expected: &TokenKind) -> bool {
        self.check_simple(expected)
    }

    fn check_identifier(&self) -> bool {
        self.peek().is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))
    }

    fn peek_next_is(&self, expected: &TokenKind) -> bool {
        self.tokens.get(self.cursor + 1).is_some_and(|token| token.kind == *expected)
    }

    fn advance_required(&mut self, expected: &str) -> Result<Token, ParseError> {
        if let Some(token) = self.tokens.get(self.cursor).cloned() {
            self.cursor += 1;
            Ok(token)
        } else {
            Err(ParseError {
                code: ParseErrorCode::UnexpectedEndOfInput,
                kind: ParseErrorKind::UnexpectedEndOfInput { expected: expected.to_owned() },
                span: SourceSpan::new(0, 0),
            })
        }
    }

    fn error_at_current(&self, expected: &str) -> ParseError {
        match self.peek() {
            Some(token) => ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: expected.to_owned(),
                    found: token.kind.clone(),
                },
                span: token.span,
            },
            None => ParseError {
                code: ParseErrorCode::UnexpectedEndOfInput,
                kind: ParseErrorKind::UnexpectedEndOfInput { expected: expected.to_owned() },
                span: SourceSpan::new(0, 0),
            },
        }
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.cursor - 1]
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn at_end(&self) -> bool {
        self.check_simple(&TokenKind::Eof)
    }
}

fn identifier_text(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Identifier(name) => name.clone(),
        _ => unreachable!("identifier_text called with a non-identifier token"),
    }
}

fn role_from_token(kind: &TokenKind) -> Option<Role> {
    match kind {
        TokenKind::Erg => Some(Role::Erg),
        TokenKind::Abs => Some(Role::Abs),
        TokenKind::Dat => Some(Role::Dat),
        _ => None,
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. } => *span,
    }
}
