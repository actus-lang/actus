use crate::ast::{
    Block, Expr, ExternalVerbDecl, ForeignAbi, Param, Program, Role, Stmt, TopLevelDecl, TypeName,
    VerbDecl,
};
use crate::lexer::{SourceSpan, Token, TokenKind};
use std::collections::HashSet;

mod cursor;
mod enums;
mod expressions;
mod structs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedToken { expected: String, found: TokenKind },
    UnexpectedEndOfInput { expected: String },
    DuplicateName { kind: String, name: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseErrorCode {
    UnexpectedToken,
    UnexpectedEndOfInput,
    DuplicateName,
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
    enum_names: HashSet<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0, enum_names: HashSet::new() }
    }

    pub fn parse(mut self) -> Result<Program, ParseError> {
        let mut declarations = Vec::new();

        while !self.at_end() {
            declarations.push(self.parse_top_level_decl()?);
        }

        Ok(Program { declarations })
    }

    fn parse_top_level_decl(&mut self) -> Result<TopLevelDecl, ParseError> {
        if self.check_simple(&TokenKind::Unsafe) {
            return Ok(TopLevelDecl::ExternalVerb(self.parse_external_verb(true)?));
        }
        if self.check_simple(&TokenKind::Extern) {
            return Ok(TopLevelDecl::ExternalVerb(self.parse_external_verb(false)?));
        }
        if self.check_simple(&TokenKind::Struct) {
            return Ok(TopLevelDecl::Struct(self.parse_struct_def()?));
        }
        if self.check_simple(&TokenKind::Enum) {
            return Ok(TopLevelDecl::Enum(self.parse_enum_def()?));
        }
        let declaration = self.parse_verb()?;
        Ok(TopLevelDecl::Verb(declaration))
    }

    fn parse_external_verb(
        &mut self,
        unsafe_boundary: bool,
    ) -> Result<ExternalVerbDecl, ParseError> {
        let start = if unsafe_boundary {
            let start = self.expect_keyword(TokenKind::Unsafe, "`unsafe`")?.span.start;
            self.expect_keyword(TokenKind::Extern, "`extern`")?;
            start
        } else {
            self.expect_keyword(TokenKind::Extern, "`extern`")?.span.start
        };
        let abi_token = self.advance_required("ABI string")?;
        let abi = match &abi_token.kind {
            TokenKind::StringLiteral(abi) if abi == "C" => ForeignAbi::C,
            _ => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "supported ABI string (currently `\"C\"`)".to_owned(),
                        found: abi_token.kind.clone(),
                    },
                    span: abi_token.span,
                });
            }
        };
        self.expect_keyword(TokenKind::Verb, "`verb`")?;
        let name_token = self.take_identifier("external verb name")?;
        let name = identifier_text(&name_token.kind);
        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RightParen, "`)`")?;
        let return_type =
            if self.match_simple(TokenKind::Arrow) { Some(self.parse_type_name()?) } else { None };
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(ExternalVerbDecl {
            unsafe_boundary,
            abi,
            name,
            params,
            return_type,
            span: SourceSpan::new(start, end),
        })
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
        if self.match_simple(TokenKind::Equals) {
            let value = self.parse_expression()?;
            let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
            let span = SourceSpan::new(expression_span(&expression).start, end);
            return match expression {
                Expr::Identifier { name, .. } => Ok(Stmt::Assignment { name, value, span }),
                Expr::FieldAccess { object, field, .. } => {
                    Ok(Stmt::FieldAssignment { object: *object, field, value, span })
                }
                _ => Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "assignable binding or field".to_owned(),
                        found: self
                            .peek()
                            .map(|token| token.kind.clone())
                            .unwrap_or(TokenKind::Eof),
                    },
                    span,
                }),
            };
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
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. } => *span,
    }
}
