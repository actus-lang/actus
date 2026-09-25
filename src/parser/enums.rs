use std::collections::HashSet;

use crate::ast::{EnumDef, EnumField, EnumPayload, EnumVariant};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, identifier_text};

impl Parser {
    pub(super) fn parse_enum_def(&mut self, is_open: bool) -> Result<EnumDef, ParseError> {
        let start = self.expect_keyword(TokenKind::Enum, "`enum`")?.span.start;
        let name_token = self.take_identifier("enum name")?;
        let name = identifier_text(&name_token.kind);
        let generic_parameters = self.parse_generic_parameters()?;
        if !self.enum_names.insert(name.clone()) {
            return Err(duplicate_name("enum", name, name_token.span));
        }
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;
        let variants = self.parse_enum_variants()?;
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(EnumDef {
            is_open,
            name,
            generic_parameters,
            variants,
            span: SourceSpan::new(start, end),
        })
    }

    fn parse_enum_variants(&mut self) -> Result<Vec<EnumVariant>, ParseError> {
        let mut variants = Vec::new();
        let mut names = HashSet::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            if self.at_end() {
                return Err(self.error_at_current("`}`"));
            }
            let variant = self.parse_enum_variant()?;
            if !names.insert(variant.name.clone()) {
                return Err(duplicate_name("enum variant", variant.name, variant.span));
            }
            variants.push(variant);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        Ok(variants)
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariant, ParseError> {
        let name_token = self.take_identifier("enum variant name")?;
        let name = identifier_text(&name_token.kind);
        let payload = if self.match_simple(TokenKind::LeftParen) {
            EnumPayload::Tuple(self.parse_tuple_payload()?)
        } else if self.match_simple(TokenKind::LeftBrace) {
            EnumPayload::Struct(self.parse_named_payload()?)
        } else {
            EnumPayload::Unit
        };
        let end = self.previous().span.end;
        Ok(EnumVariant { name, payload, span: SourceSpan::new(name_token.span.start, end) })
    }

    fn parse_tuple_payload(&mut self) -> Result<Vec<crate::ast::TypeName>, ParseError> {
        let mut fields = Vec::new();
        while !self.check_simple(&TokenKind::RightParen) {
            fields.push(self.parse_type_name()?);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "`)`")?;
        Ok(fields)
    }

    fn parse_named_payload(&mut self) -> Result<Vec<EnumField>, ParseError> {
        let mut fields = Vec::new();
        let mut names = HashSet::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            let name_token = self.take_identifier("enum payload field name")?;
            let name = identifier_text(&name_token.kind);
            if !names.insert(name.clone()) {
                return Err(duplicate_name("enum payload field", name, name_token.span));
            }
            self.expect_simple(TokenKind::Colon, "`:`")?;
            let ty = self.parse_type_name()?;
            fields.push(EnumField {
                name,
                span: SourceSpan::new(name_token.span.start, ty.span.end),
                ty,
            });
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightBrace, "`}`")?;
        Ok(fields)
    }
}

fn duplicate_name(kind: &str, name: String, span: SourceSpan) -> ParseError {
    ParseError {
        code: ParseErrorCode::DuplicateName,
        kind: ParseErrorKind::DuplicateName { kind: kind.to_owned(), name },
        span,
    }
}
