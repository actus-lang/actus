use std::collections::HashSet;

use crate::ast::{GenericParam, GenericParamKind, TypeName};
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
            let kind = match bound.as_ref() {
                Some(domain) if domain.name == "Usize" && domain.arguments.is_empty() => {
                    GenericParamKind::Const { domain: domain.clone() }
                }
                _ => GenericParamKind::Type,
            };
            let end = bounds.last().map_or(name_token.span.end, |ty| ty.span.end);
            parameters.push(GenericParam {
                name,
                kind,
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
        self.parse_type_name_with_reference_role(false)
    }

    fn parse_type_name_with_reference_role(
        &mut self,
        allow_reference_role: bool,
    ) -> Result<TypeName, ParseError> {
        let reference_role =
            if allow_reference_role { self.parse_type_reference_role()? } else { None };
        let token = self.advance_required("type name")?;
        let name = type_name_text(&token.kind).ok_or_else(|| ParseError {
            code: ParseErrorCode::UnexpectedToken,
            kind: ParseErrorKind::UnexpectedToken {
                expected: "type name".to_owned(),
                found: token.kind.clone(),
            },
            span: token.span,
        })?;
        let arguments = if self.match_simple(TokenKind::LeftBracket) {
            self.parse_type_arguments()?
        } else {
            Vec::new()
        };
        let end = self.previous().span.end;
        self.validate_bounded_array(name.as_str(), &arguments)?;
        Ok(TypeName {
            name,
            arguments,
            reference_role,
            span: SourceSpan::new(token.span.start, end),
        })
    }

    fn validate_bounded_array(&self, name: &str, arguments: &[TypeName]) -> Result<(), ParseError> {
        if name != "Array" || arguments.is_empty() {
            return Ok(());
        }
        if arguments.len() != 2 {
            return Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "Array element type and capacity".to_owned(),
                    found: TokenKind::Eof,
                },
                span: self.previous().span,
            });
        }
        let capacity = &arguments[1];
        if !capacity.arguments.is_empty() || capacity.reference_role.is_some() {
            return Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: "an integer literal or const generic array capacity".to_owned(),
                    found: TokenKind::Identifier(capacity.name.clone()),
                },
                span: capacity.span,
            });
        }
        Ok(())
    }

    fn parse_type_reference_role(&mut self) -> Result<Option<crate::ast::Role>, ParseError> {
        if self.check_simple(&TokenKind::Abs) {
            self.expect_simple(TokenKind::Abs, "`abs`")?;
            return Ok(Some(crate::ast::Role::Abs));
        }
        if self.check_simple(&TokenKind::Ins) {
            self.expect_simple(TokenKind::Ins, "`ins`")?;
            return Ok(Some(crate::ast::Role::Ins));
        }
        Ok(None)
    }

    pub(super) fn parse_type_arguments(&mut self) -> Result<Vec<TypeName>, ParseError> {
        let mut arguments = Vec::new();
        if self.check_simple(&TokenKind::RightBracket) {
            return Err(self.error_at_current("a type argument"));
        }
        loop {
            arguments.push(self.parse_type_name_with_reference_role(true)?);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightBracket, "`]`")?;
        Ok(arguments)
    }
}

fn type_name_text(kind: &TokenKind) -> Option<String> {
    match kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        TokenKind::Integer { value, suffix: None } => Some(value.clone()),
        TokenKind::IntType { signed, width } => {
            Some(format!("{}{}", if *signed { 'i' } else { 'u' }, width))
        }
        TokenKind::FloatType { width } => Some(format!("f{width}")),
        TokenKind::VoidType => Some("Void".to_owned()),
        _ => None,
    }
}

fn duplicate_name(kind: &str, name: String, span: SourceSpan) -> ParseError {
    ParseError {
        code: ParseErrorCode::DuplicateName,
        kind: ParseErrorKind::DuplicateName { kind: kind.to_owned(), name },
        span,
    }
}
