use crate::ast::{Expr, StructDef, StructField, StructFieldInit, StructFieldRole};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, expression_span, identifier_text};

impl Parser {
    pub(super) fn parse_struct_def(&mut self) -> Result<StructDef, ParseError> {
        let start = self.expect_keyword(TokenKind::Struct, "`struct`")?.span.start;
        let name_token = self.take_identifier("struct name")?;
        let name = identifier_text(&name_token.kind);
        let generic_parameters = self.parse_generic_parameters()?;
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;
        let fields = self.parse_struct_fields()?;
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(StructDef { name, generic_parameters, fields, span: SourceSpan::new(start, end) })
    }

    fn parse_struct_fields(&mut self) -> Result<Vec<StructField>, ParseError> {
        let mut fields = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            if self.at_end() {
                return Err(self.error_at_current("`}`"));
            }
            fields.push(self.parse_struct_field()?);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_struct_field(&mut self) -> Result<StructField, ParseError> {
        let first = self.advance_required("struct field")?;
        let (role, start) = match first.kind {
            TokenKind::Erg => (StructFieldRole::Erg, first.span.start),
            TokenKind::Abs | TokenKind::Dat => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "a value field or an `erg` field".to_owned(),
                        found: first.kind,
                    },
                    span: first.span,
                });
            }
            TokenKind::Identifier(_) => (StructFieldRole::Value, first.span.start),
            found => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "a struct field name".to_owned(),
                        found,
                    },
                    span: first.span,
                });
            }
        };
        let name = if matches!(first.kind, TokenKind::Identifier(_)) {
            identifier_text(&first.kind)
        } else {
            identifier_text(&self.take_identifier("struct field name")?.kind)
        };
        self.expect_simple(TokenKind::Colon, "`:`")?;
        let ty = self.parse_type_name()?;
        Ok(StructField { role, name, span: SourceSpan::new(start, ty.span.end), ty })
    }

    pub(super) fn parse_struct_literal(
        &mut self,
        name: String,
        type_arguments: Vec<crate::ast::TypeName>,
        start: usize,
    ) -> Result<Expr, ParseError> {
        let mut fields = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            if self.at_end() {
                return Err(self.error_at_current("`}`"));
            }
            let field_token = self.take_identifier("struct initializer field name")?;
            let field_name = identifier_text(&field_token.kind);
            self.expect_simple(TokenKind::Colon, "`:`")?;
            let value = self.parse_expression()?;
            let span = SourceSpan::new(field_token.span.start, expression_span(&value).end);
            fields.push(StructFieldInit { name: field_name, value, span });
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(Expr::StructLit { name, type_arguments, fields, span: SourceSpan::new(start, end) })
    }
}
