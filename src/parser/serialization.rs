use crate::ast::{LayoutEndianness, SerializeDecl, SerializeSection};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, identifier_text};

impl Parser {
    pub(super) fn parse_serialize_decl(
        &mut self,
        is_open: bool,
        doc: Option<String>,
    ) -> Result<SerializeDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Serialize, "`serialize`")?.span.start;
        let name = identifier_text(&self.take_identifier("serialization contract name")?.kind);
        self.expect_serialization_word("from")?;
        let source_type = self.parse_type_name()?;
        self.expect_simple(TokenKind::LeftBrace, "`{")?;
        self.expect_serialization_word("layout")?;
        let endianness = self.parse_serialization_endianness()?;
        self.expect_simple(TokenKind::Semicolon, "`;`")?;
        let mut sections = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            sections.push(self.parse_serialize_section()?);
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(SerializeDecl {
            is_open,
            doc,
            name,
            source_type,
            endianness,
            sections,
            span: SourceSpan::new(start, end),
        })
    }

    fn parse_serialize_section(&mut self) -> Result<SerializeSection, ParseError> {
        let token = self.take_identifier("serialization section")?;
        let start = token.span.start;
        match identifier_text(&token.kind).as_str() {
            "version" => {
                let ty = self.parse_type_name()?;
                self.expect_serialization_word("at")?;
                let offset = self.parse_serialization_u16("version byte offset")?;
                let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
                Ok(SerializeSection::Version { ty, offset, span: SourceSpan::new(start, end) })
            }
            "payload" => {
                self.expect_serialization_word("bytes")?;
                self.expect_serialization_word("at")?;
                let offset = self.parse_serialization_u16("payload byte offset")?;
                self.expect_serialization_word("length")?;
                let length = self.parse_serialization_u16("payload byte length")?;
                let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
                Ok(SerializeSection::Payload { offset, length, span: SourceSpan::new(start, end) })
            }
            "checksum" => {
                self.expect_serialization_word("crc32")?;
                self.expect_serialization_word("over")?;
                let range_start = self.parse_serialization_u16("checksum range start")?;
                self.expect_simple(TokenKind::DotDot, "`..`")?;
                let range_end = self.parse_serialization_u16("checksum range end")?;
                self.expect_serialization_word("at")?;
                let offset = self.parse_serialization_u16("checksum byte offset")?;
                let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
                Ok(SerializeSection::Checksum {
                    start: range_start,
                    end: range_end,
                    offset,
                    span: SourceSpan::new(start, end),
                })
            }
            _ => Err(self.serialization_error(token.span, "`version`, `payload`, or `checksum`")),
        }
    }

    fn parse_serialization_endianness(&mut self) -> Result<LayoutEndianness, ParseError> {
        let token = self.take_identifier("`little` or `big`")?;
        match identifier_text(&token.kind).as_str() {
            "little" => Ok(LayoutEndianness::Little),
            "big" => Ok(LayoutEndianness::Big),
            _ => Err(self.serialization_error(token.span, "`little` or `big`")),
        }
    }

    fn parse_serialization_u16(&mut self, expected: &str) -> Result<u16, ParseError> {
        let token = self.advance_required(expected)?;
        match token.kind {
            TokenKind::Integer { value, suffix: None } => {
                value.parse::<u16>().map_err(|_| self.serialization_error(token.span, expected))
            }
            _ => Err(self.serialization_error(token.span, expected)),
        }
    }

    fn expect_serialization_word(&mut self, expected: &str) -> Result<(), ParseError> {
        let token = self.take_identifier(expected)?;
        if identifier_text(&token.kind) == expected {
            Ok(())
        } else {
            Err(self.serialization_error(token.span, expected))
        }
    }

    fn serialization_error(&self, span: SourceSpan, expected: &str) -> ParseError {
        ParseError {
            code: ParseErrorCode::UnexpectedToken,
            kind: ParseErrorKind::UnexpectedToken {
                expected: expected.to_owned(),
                found: TokenKind::Eof,
            },
            span,
        }
    }
}
