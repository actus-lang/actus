use std::collections::HashSet;

use crate::ast::{GenericParam, TypeName};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, identifier_text};

impl Parser {
    pub(super) fn parse_generic_parameters(&mut self) -> Result<Vec<GenericParam>, ParseError> {
        if !self.match_simple(TokenKind::LeftBracket) {
            return Ok(Vec::new());
        }

        let mut parameters = Vec::new();
        let mut names = HashSet::new();
        while !self.check_simple(&TokenKind::RightBracket) {
            let name_token = self.take_identifier("generic parameter name")?;
            let name = identifier_text(&name_token.kind);
            if !names.insert(name.clone()) {
                return Err(duplicate_name("generic parameter", name, name_token.span));
            }
            let bounds = if self.match_simple(TokenKind::Colon) {
                let mut bounds = vec![self.parse_type_name()?];
                while self.match_simple(TokenKind::Plus) {
                    bounds.push(self.parse_type_name()?);
                }
                bounds
            } else {
                Vec::new()
            };
            let bound = bounds.first().cloned();
            let end = bounds.last().map_or(name_token.span.end, |ty| ty.span.end);
            parameters.push(GenericParam {
                name,
                bound,
                bounds,
                span: SourceSpan::new(name_token.span.start, end),
            });
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightBracket, "`]`")?;
        Ok(parameters)
    }

    pub(super) fn parse_type_name(&mut self) -> Result<TypeName, ParseError> {
        let token = self.take_identifier("type name")?;
        let name = identifier_text(&token.kind);
        let arguments = if self.match_simple(TokenKind::LeftBracket) {
            self.parse_type_arguments()?
        } else {
            Vec::new()
        };
        let end = self.previous().span.end;
        Ok(TypeName { name, arguments, span: SourceSpan::new(token.span.start, end) })
    }

    pub(super) fn parse_type_arguments(&mut self) -> Result<Vec<TypeName>, ParseError> {
        let mut arguments = Vec::new();
        if self.check_simple(&TokenKind::RightBracket) {
            return Err(self.error_at_current("a type argument"));
        }
        loop {
            arguments.push(self.parse_type_name()?);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightBracket, "`]`")?;
        Ok(arguments)
    }
}

fn duplicate_name(kind: &str, name: String, span: SourceSpan) -> ParseError {
    ParseError {
        code: ParseErrorCode::DuplicateName,
        kind: ParseErrorKind::DuplicateName { kind: kind.to_owned(), name },
        span,
    }
}
