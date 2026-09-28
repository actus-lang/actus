use crate::ast::{PerformDecl, RoleDecl, RoleMethod};
use crate::lexer::{SourceSpan, TokenKind};

use super::{ParseError, Parser, identifier_text};

impl Parser {
    pub(super) fn parse_role_decl(
        &mut self,
        is_open: bool,
        doc: Option<String>,
    ) -> Result<RoleDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Role, "`role`")?.span.start;
        let name_token = self.take_identifier("role name")?;
        let name = identifier_text(&name_token.kind);
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;
        let mut methods = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            let doc = self.take_doc_string_group();
            methods.push(self.parse_role_method(doc)?);
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(RoleDecl { is_open, doc, name, methods, span: SourceSpan::new(start, end) })
    }

    fn parse_role_method(&mut self, doc: Option<String>) -> Result<RoleMethod, ParseError> {
        let start = self.expect_keyword(TokenKind::Verb, "`verb`")?.span.start;
        let name_token = self.take_callable_name("role method name")?;
        let name = identifier_text(&name_token.kind);
        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RightParen, "`)`")?;
        let return_type = self.parse_return_type()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(RoleMethod { doc, name, params, return_type, span: SourceSpan::new(start, end) })
    }

    pub(super) fn parse_perform_decl(
        &mut self,
        is_open: bool,
        doc: Option<String>,
    ) -> Result<PerformDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Perform, "`perform`")?.span.start;
        let role_token = self.take_identifier("role name after `perform`")?;
        let role_name = identifier_text(&role_token.kind);
        self.expect_keyword(TokenKind::For, "`for`")?;
        let target = self.parse_type_name()?;
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;
        let mut methods = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            let method_doc = self.take_doc_string_group();
            methods.push(self.parse_verb_with_metadata(false, Vec::new(), method_doc)?);
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(PerformDecl {
            is_open,
            doc,
            role_name,
            target,
            methods,
            span: SourceSpan::new(start, end),
        })
    }
}
