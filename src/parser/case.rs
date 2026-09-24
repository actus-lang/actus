use crate::ast::{
    CaseBody, CaseBranch, CaseMode, LiteralPattern, NamedPattern, Pattern, PatternBinding, Stmt,
    VariantPayload,
};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, expression_span, identifier_text};

impl Parser {
    pub(super) fn parse_case_expression(&mut self) -> Result<crate::ast::Expr, ParseError> {
        let start = self.previous().span.start;
        let mode = if self.match_simple(TokenKind::Dat) {
            CaseMode::Dat
        } else {
            self.match_simple(TokenKind::Abs);
            CaseMode::Abs
        };
        let previous_case_subject = self.case_subject;
        self.case_subject = true;
        let subject = self.parse_expression()?;
        self.case_subject = previous_case_subject;
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;
        let mut branches = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            if self.at_end() {
                return Err(self.error_at_current("`}`"));
            }
            branches.push(self.parse_case_branch()?);
            if !self.match_simple(TokenKind::Comma) && !self.check_simple(&TokenKind::RightBrace) {
                return Err(self.error_at_current("`,` or `}`"));
            }
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(crate::ast::Expr::Case {
            mode,
            subject: Box::new(subject),
            branches,
            span: SourceSpan::new(start, end),
        })
    }

    fn parse_case_branch(&mut self) -> Result<CaseBranch, ParseError> {
        let pattern = self.parse_pattern()?;
        let start = pattern_span(&pattern).start;
        let guard = if self.match_simple(TokenKind::If) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        self.expect_simple(TokenKind::FatArrow, "`=>`")?;
        let body = if self.check_simple(&TokenKind::LeftBrace) {
            CaseBody::Block(self.parse_case_block()?)
        } else {
            CaseBody::Expression(Box::new(self.parse_expression()?))
        };
        let end = case_body_end(&body);
        Ok(CaseBranch { pattern, guard, body, span: SourceSpan::new(start, end) })
    }

    fn parse_case_block(&mut self) -> Result<crate::ast::Block, ParseError> {
        let start = self.expect_simple(TokenKind::LeftBrace, "`{`")?.span.start;
        let mut statements = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            if self.at_end() {
                return Err(self.error_at_current("`}`"));
            }
            let statement = self.parse_statement()?;
            if let Stmt::Break { span } = statement {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "case branch body without standalone `break`".to_owned(),
                        found: TokenKind::Break,
                    },
                    span,
                });
            }
            statements.push(statement);
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(crate::ast::Block { statements, span: SourceSpan::new(start, end) })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let token = self.advance_required("case pattern")?;
        match token.kind {
            TokenKind::Underscore => Ok(Pattern::Wildcard { span: token.span }),
            TokenKind::Integer(value) => {
                Ok(Pattern::Literal { value: LiteralPattern::Integer(value), span: token.span })
            }
            TokenKind::True | TokenKind::False => Ok(Pattern::Literal {
                value: LiteralPattern::Bool(matches!(token.kind, TokenKind::True)),
                span: token.span,
            }),
            TokenKind::Identifier(enum_name) => self.parse_variant_pattern(enum_name, token.span),
            found => Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "variant, integer, boolean, or `_` pattern".to_owned(),
                    found,
                },
                span: token.span,
            }),
        }
    }

    fn parse_variant_pattern(
        &mut self,
        enum_name: String,
        start: SourceSpan,
    ) -> Result<Pattern, ParseError> {
        self.expect_simple(TokenKind::Dot, "`.`")?;
        let variant_token = self.take_identifier("variant name")?;
        let variant = identifier_text(&variant_token.kind);
        let (payload, end) = if self.match_simple(TokenKind::LeftParen) {
            let named = self.check_identifier() && self.peek_next_is(&TokenKind::Colon);
            let payload = if named {
                VariantPayload::Named(self.parse_named_patterns(TokenKind::RightParen)?)
            } else {
                VariantPayload::Positional(self.parse_pattern_bindings(TokenKind::RightParen)?)
            };
            let end = self.expect_simple(TokenKind::RightParen, "`)`")?.span.end;
            (payload, end)
        } else if self.match_simple(TokenKind::LeftBrace) {
            let fields = self.parse_named_patterns(TokenKind::RightBrace)?;
            let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
            (VariantPayload::Named(fields), end)
        } else {
            (VariantPayload::Unit, variant_token.span.end)
        };
        Ok(Pattern::Variant {
            enum_name,
            variant,
            payload,
            span: SourceSpan::new(start.start, end),
        })
    }

    fn parse_pattern_bindings(
        &mut self,
        terminator: TokenKind,
    ) -> Result<Vec<PatternBinding>, ParseError> {
        let mut bindings = Vec::new();
        if self.check_simple(&terminator) {
            return Ok(bindings);
        }
        loop {
            bindings.push(self.parse_pattern_binding()?);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        Ok(bindings)
    }

    fn parse_named_patterns(
        &mut self,
        terminator: TokenKind,
    ) -> Result<Vec<NamedPattern>, ParseError> {
        let mut fields = Vec::new();
        if self.check_simple(&terminator) {
            return Ok(fields);
        }
        loop {
            let field_token = self.take_identifier("pattern field name")?;
            let name = identifier_text(&field_token.kind);
            self.expect_simple(TokenKind::Colon, "`:`")?;
            let binding = self.parse_pattern_binding()?;
            let span = SourceSpan::new(field_token.span.start, binding.span.end);
            fields.push(NamedPattern { name, binding, span });
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_pattern_binding(&mut self) -> Result<PatternBinding, ParseError> {
        let token = self.advance_required("pattern binding")?;
        let name = match token.kind {
            TokenKind::Identifier(name) => name,
            TokenKind::Underscore => "_".to_owned(),
            found => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "pattern binding identifier".to_owned(),
                        found,
                    },
                    span: token.span,
                });
            }
        };
        Ok(PatternBinding { name, span: token.span })
    }
}

fn pattern_span(pattern: &Pattern) -> SourceSpan {
    match pattern {
        Pattern::Variant { span, .. }
        | Pattern::Literal { span, .. }
        | Pattern::Wildcard { span } => *span,
    }
}

fn case_body_end(body: &CaseBody) -> usize {
    match body {
        CaseBody::Expression(expression) => expression_span(expression).end,
        CaseBody::Block(block) => block.span.end,
    }
}
