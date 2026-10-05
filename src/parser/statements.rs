use crate::ast::{Block, CompoundAssignmentOp, Expr, ForBinding, Place, Role, Stmt};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, expression_span, identifier_text};

impl Parser {
    pub(super) fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.parse_block_with_statement_conditionals(true)
    }

    pub(super) fn parse_block_with_statement_conditionals(
        &mut self,
        allow_statement_conditionals: bool,
    ) -> Result<Block, ParseError> {
        self.parse_block_with_options(allow_statement_conditionals, false)
    }

    pub(super) fn parse_expression_block(&mut self) -> Result<Block, ParseError> {
        self.parse_block_with_options(false, true)
    }

    fn parse_block_with_options(
        &mut self,
        allow_statement_conditionals: bool,
        allow_implicit_final_expression: bool,
    ) -> Result<Block, ParseError> {
        let start = self.expect_simple(TokenKind::LeftBrace, "`{`")?.span.start;
        let mut statements = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            if self.at_end() {
                return Err(self.error_at_current("`}`"));
            }
            self.skip_doc_strings();
            if self.check_simple(&TokenKind::RightBrace) {
                break;
            }
            statements.push(self.parse_statement_with_options(
                allow_statement_conditionals,
                allow_implicit_final_expression,
            )?);
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(Block { statements, span: SourceSpan::new(start, end) })
    }

    pub(super) fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        self.parse_statement_with_options(true, false)
    }

    fn parse_statement_with_options(
        &mut self,
        allow_statement_conditionals: bool,
        allow_implicit_final_expression: bool,
    ) -> Result<Stmt, ParseError> {
        if self.check_simple(&TokenKind::LeftBrace) {
            return Ok(Stmt::Block(self.parse_block()?));
        }
        if self.match_simple(TokenKind::Loop) {
            return Ok(Stmt::Loop(self.parse_block()?));
        }
        if self.match_simple(TokenKind::For) {
            return self.parse_for_range_statement();
        }
        if allow_statement_conditionals && self.match_simple(TokenKind::If) {
            let statement = self.parse_if_statement(self.previous().span)?;
            self.match_simple(TokenKind::Semicolon);
            return Ok(statement);
        }
        if self.check_role(&TokenKind::Erg)
            || self.check_role(&TokenKind::Abs)
            || self.check_role(&TokenKind::Ins)
        {
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
        self.parse_expression_statement(expression, allow_implicit_final_expression)
    }

    fn parse_for_range_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.previous().span.start;
        let role_token = self.advance_required("loop binding role")?;
        let role = match role_token.kind {
            TokenKind::Erg => Role::Erg,
            TokenKind::Abs => Role::Abs,
            TokenKind::Ins => Role::Ins,
            found => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "`erg`, `abs`, or `ins` loop binding role".to_owned(),
                        found,
                    },
                    span: role_token.span,
                });
            }
        };
        let name_token = self.take_identifier("loop binding name")?;
        let name = identifier_text(&name_token.kind);
        let ty = if self.match_simple(TokenKind::Colon) {
            Some(format_type_name(&self.parse_type_name()?))
        } else {
            None
        };
        self.expect_simple(TokenKind::In, "`in`")?;
        let range_start = self.parse_expression()?;
        let binding = ForBinding {
            role,
            name,
            ty,
            span: SourceSpan::new(role_token.span.start, name_token.span.end),
        };
        if self.match_simple(TokenKind::DotDot) {
            let range_end = self.parse_expression()?;
            let body = self.parse_block()?;
            let span = SourceSpan::new(start, body.span.end);
            return Ok(Stmt::ForRange { binding, start: range_start, end: range_end, body, span });
        }
        let body = self.parse_block()?;
        let span = SourceSpan::new(start, body.span.end);
        Ok(Stmt::ForArray { binding, collection: range_start, body, span })
    }

    fn parse_expression_statement(
        &mut self,
        expression: Expr,
        allow_implicit_final_expression: bool,
    ) -> Result<Stmt, ParseError> {
        if self.match_simple(TokenKind::Equals) {
            return self.parse_assignment(expression);
        }
        if let Some(operator) = self.match_compound_assignment() {
            return self.parse_compound_assignment(expression, operator);
        }
        if allow_implicit_final_expression {
            let span = expression_span(&expression);
            if self.check_simple(&TokenKind::RightBrace) {
                return Ok(Stmt::Expression { expression, span });
            }
            if self.match_simple(TokenKind::Semicolon) {
                if self.check_simple(&TokenKind::RightBrace) {
                    return Ok(Stmt::Expression { expression, span });
                }
                return Ok(Stmt::Expression {
                    expression,
                    span: SourceSpan::new(span.start, self.previous().span.end),
                });
            }
        }
        let start = expression_span(&expression).start;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(Stmt::Expression { expression, span: SourceSpan::new(start, end) })
    }

    fn match_compound_assignment(&mut self) -> Option<CompoundAssignmentOp> {
        let operator = match self.peek()?.kind {
            TokenKind::PlusEquals => CompoundAssignmentOp::Add,
            TokenKind::MinusEquals => CompoundAssignmentOp::Subtract,
            TokenKind::StarEquals => CompoundAssignmentOp::Multiply,
            TokenKind::SlashEquals => CompoundAssignmentOp::Divide,
            TokenKind::PercentEquals => CompoundAssignmentOp::Remainder,
            TokenKind::AmpersandEquals => CompoundAssignmentOp::BitwiseAnd,
            TokenKind::PipeEquals => CompoundAssignmentOp::BitwiseOr,
            TokenKind::CaretEquals => CompoundAssignmentOp::BitwiseXor,
            TokenKind::ShiftLeftEquals => CompoundAssignmentOp::ShiftLeft,
            TokenKind::ShiftRightEquals => CompoundAssignmentOp::ShiftRight,
            _ => return None,
        };
        self.advance_required("compound assignment operator").ok()?;
        Some(operator)
    }

    fn parse_compound_assignment(
        &mut self,
        expression: Expr,
        operator: CompoundAssignmentOp,
    ) -> Result<Stmt, ParseError> {
        let value = self.parse_expression()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        let span = SourceSpan::new(expression_span(&expression).start, end);
        let target = self.parse_place(expression, span)?;
        Ok(Stmt::CompoundAssignment { target, operator, value, span })
    }

    fn parse_assignment(&mut self, expression: Expr) -> Result<Stmt, ParseError> {
        let value = self.parse_expression()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        let span = SourceSpan::new(expression_span(&expression).start, end);
        let target = self.parse_place(expression, span)?;
        Ok(Stmt::Assignment { target, value, span })
    }

    fn parse_place(&self, expression: Expr, span: SourceSpan) -> Result<Place, ParseError> {
        match expression {
            Expr::Identifier { name, span } => Ok(Place::Binding { name, span }),
            Expr::FieldAccess { object, field, span } => {
                Ok(Place::Field { object: Box::new(self.parse_place(*object, span)?), field, span })
            }
            Expr::Index { target, index, span } => Ok(Place::Index {
                target: Box::new(self.parse_place(*target, span)?),
                index: *index,
                span,
            }),
            _ => Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "assignable binding, field, or index".to_owned(),
                    found: self.peek().map(|token| token.kind.clone()).unwrap_or(TokenKind::Eof),
                },
                span,
            }),
        }
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
        let (role, name, ty, start) = self.parse_owner_header()?;
        self.expect_simple(TokenKind::Equals, "`=`")?;
        let initializer = self.parse_owner_initializer(&role, start)?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(Stmt::OwnerDecl { role, name, ty, initializer, span: SourceSpan::new(start, end) })
    }

    fn parse_owner_header(&mut self) -> Result<(Role, String, Option<String>, usize), ParseError> {
        let role_token = self.advance_required("binding role")?;
        let role = match role_token.kind {
            TokenKind::Erg => Role::Erg,
            TokenKind::Abs => Role::Abs,
            TokenKind::Ins => Role::Ins,
            found => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "`erg`, `abs`, or `ins`".to_owned(),
                        found,
                    },
                    span: role_token.span,
                });
            }
        };
        let name_token = self.take_identifier("binding name")?;
        let name = identifier_text(&name_token.kind);
        let ty = if self.match_simple(TokenKind::Colon) {
            Some(format_type_name(&self.parse_type_name()?))
        } else {
            None
        };
        Ok((role, name, ty, role_token.span.start))
    }

    fn parse_owner_initializer(&mut self, role: &Role, start: usize) -> Result<Expr, ParseError> {
        let initializer = if *role == Role::Abs {
            self.expect_simple(TokenKind::Ref, "`ref`")?;
            let expression = self.parse_expression()?;
            let span = SourceSpan::new(start, expression_span(&expression).end);
            Expr::Borrow { expression: Box::new(expression), span }
        } else {
            self.parse_expression()?
        };
        Ok(initializer)
    }
}

fn format_type_name(type_name: &crate::ast::TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(format_type_name).collect::<Vec<_>>().join(",")
    )
}
