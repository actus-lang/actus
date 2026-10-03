use crate::ast::{LayoutEndianness, PackDecl, PackField, PackStorage, Role, TypeName};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, identifier_text};

impl Parser {
    pub(super) fn parse_pack_decl(
        &mut self,
        is_open: bool,
        doc: Option<String>,
    ) -> Result<PackDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Pack, "`pack`")?.span.start;
        let name = identifier_text(&self.take_identifier("pack name")?.kind);
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;

        let (storage_doc, storage_name, storage) = self.parse_pack_storage()?;
        let (endianness, fields, end) = self.parse_pack_layout()?;

        Ok(PackDecl {
            is_open,
            doc,
            name,
            storage_name,
            storage_doc,
            storage: classify_storage(storage),
            endianness,
            fields,
            span: SourceSpan::new(start, end),
        })
    }

    fn parse_pack_layout(
        &mut self,
    ) -> Result<(LayoutEndianness, Vec<PackField>, usize), ParseError> {
        self.expect_named_identifier("layout")?;
        let endianness = self.parse_endianness()?;
        self.expect_simple(TokenKind::Semicolon, "`;`")?;
        self.expect_named_identifier("fields")?;
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;
        let fields = self.parse_pack_fields()?;
        self.expect_simple(TokenKind::RightBrace, "`}")?;
        let end = self.expect_simple(TokenKind::RightBrace, "`}")?.span.end;
        Ok((endianness, fields, end))
    }

    fn parse_pack_storage(
        &mut self,
    ) -> Result<(Option<String>, String, crate::ast::TypeName), ParseError> {
        let storage_doc = self.take_doc_string_group();
        let storage_role = self.advance_required("pack storage role")?;
        if !matches!(storage_role.kind, TokenKind::Erg) {
            return Err(unexpected(storage_role.kind, storage_role.span, "`erg` storage role"));
        }
        let storage_name = identifier_text(&self.take_identifier("storage field name")?.kind);
        self.expect_simple(TokenKind::Colon, "`:`")?;
        let storage = self.parse_type_name()?;
        self.expect_simple(TokenKind::Semicolon, "`;`")?;
        Ok((storage_doc, storage_name, storage))
    }

    fn parse_endianness(&mut self) -> Result<LayoutEndianness, ParseError> {
        let token = self.take_identifier("`little` or `big`")?;
        match identifier_text(&token.kind).as_str() {
            "little" => Ok(LayoutEndianness::Little),
            "big" => Ok(LayoutEndianness::Big),
            _ => Err(unexpected(token.kind, token.span, "`little` or `big` layout endianness")),
        }
    }

    fn parse_pack_fields(&mut self) -> Result<Vec<PackField>, ParseError> {
        let mut fields = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            let doc = self.take_doc_string_group();
            fields.push(self.parse_pack_field(doc)?);
        }
        Ok(fields)
    }

    fn parse_pack_field(&mut self, doc: Option<String>) -> Result<PackField, ParseError> {
        let role_token = self.advance_required("pack field role")?;
        let role = match role_token.kind {
            TokenKind::Erg => Role::Erg,
            TokenKind::Abs => Role::Abs,
            found => return Err(unexpected(found, role_token.span, "`erg` or `abs` field role")),
        };
        let name = identifier_text(&self.take_identifier("pack field name")?.kind);
        self.expect_simple(TokenKind::Colon, "`:`")?;
        let ty = self.parse_type_name()?;
        self.expect_named_identifier("at")?;
        let offset_token = self.advance_required("pack field bit offset")?;
        let offset = parse_offset(&offset_token.kind).ok_or_else(|| {
            unexpected(offset_token.kind.clone(), offset_token.span, "a non-negative bit offset")
        })?;
        let default_value =
            self.match_simple(TokenKind::Equals).then(|| self.parse_expression()).transpose()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(PackField {
            doc,
            role,
            name,
            ty,
            offset,
            default_value,
            span: SourceSpan::new(role_token.span.start, end),
        })
    }

    fn expect_named_identifier(&mut self, expected: &str) -> Result<(), ParseError> {
        let token = self.take_identifier(expected)?;
        if identifier_text(&token.kind) == expected {
            Ok(())
        } else {
            Err(unexpected(token.kind, token.span, format!("`{expected}`")))
        }
    }
}

fn classify_storage(type_name: TypeName) -> PackStorage {
    if type_name.name != "Array" || type_name.arguments.len() != 2 {
        return PackStorage::Scalar(type_name);
    }
    let element = type_name.arguments[0].clone();
    let Some(capacity) = parse_capacity(&type_name.arguments[1]) else {
        return PackStorage::Scalar(type_name);
    };
    PackStorage::ByteArray { type_name, element, capacity }
}

fn parse_capacity(type_name: &TypeName) -> Option<u64> {
    type_name.name.parse::<u64>().ok()
}

fn parse_offset(kind: &TokenKind) -> Option<u16> {
    let TokenKind::Integer { value, suffix: None } = kind else { return None };
    value.parse::<u16>().ok()
}

fn unexpected(found: TokenKind, span: SourceSpan, expected: impl Into<String>) -> ParseError {
    ParseError {
        code: ParseErrorCode::UnexpectedToken,
        kind: ParseErrorKind::UnexpectedToken { expected: expected.into(), found },
        span,
    }
}
