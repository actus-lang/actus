use crate::ast::{MetaAttribute, PerformDecl, RoleDecl, RoleMethod};
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
            let metadata = if self.check_simple(&TokenKind::Meta) {
                self.parse_metadata_group()?
            } else {
                Vec::new()
            };
            let doc = doc.or_else(|| self.take_doc_string_group());
            self.reject_duplicate_metadata(&metadata)?;
            self.reject_unsupported_role_metadata(&metadata)?;
            let method = self.parse_role_method(doc, metadata.clone())?;
            if metadata.iter().any(|attribute| matches!(attribute, MetaAttribute::MustUse(_)))
                && method.return_type.is_none()
            {
                return Err(ParseError {
                    code: super::ParseErrorCode::MustUseRequiresReturn,
                    kind: super::ParseErrorKind::MustUseRequiresReturn,
                    span: method.span,
                });
            }
            methods.push(method);
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(RoleDecl { is_open, doc, name, methods, span: SourceSpan::new(start, end) })
    }

    fn parse_role_method(
        &mut self,
        doc: Option<String>,
        metadata: Vec<MetaAttribute>,
    ) -> Result<RoleMethod, ParseError> {
        let start = self.expect_keyword(TokenKind::Verb, "`verb`")?.span.start;
        let name_token = self.take_callable_name("role method name")?;
        let name = identifier_text(&name_token.kind);
        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RightParen, "`)`")?;
        let return_type = self.parse_return_type()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(RoleMethod {
            doc,
            metadata,
            name,
            params,
            return_type,
            span: SourceSpan::new(start, end),
        })
    }

    pub(super) fn parse_perform_decl(
        &mut self,
        is_open: bool,
        doc: Option<String>,
    ) -> Result<PerformDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Perform, "`perform`")?.span.start;
        let role_token = self.take_identifier("role name after `perform`")?;
        let role_name = self.parse_qualified_identifier(identifier_text(&role_token.kind))?;
        self.expect_keyword(TokenKind::For, "`for`")?;
        let target = self.parse_type_name()?;
        self.expect_simple(TokenKind::LeftBrace, "`{`")?;
        let mut methods = Vec::new();
        while !self.check_simple(&TokenKind::RightBrace) {
            let method_doc = self.take_doc_string_group();
            let metadata = if self.check_simple(&TokenKind::Meta) {
                self.parse_metadata_group()?
            } else {
                Vec::new()
            };
            let method_doc = method_doc.or_else(|| self.take_doc_string_group());
            self.reject_duplicate_metadata(&metadata)?;
            methods.push(self.parse_verb_with_metadata(false, metadata, method_doc)?);
        }
        let end = self.expect_simple(TokenKind::RightBrace, "`}`")?.span.end;
        Ok(PerformDecl {
            is_open,
            module_import: false,
            module_import_namespace: None,
            doc,
            role_name,
            target,
            methods,
            span: SourceSpan::new(start, end),
        })
    }

    fn reject_unsupported_role_metadata(
        &self,
        metadata: &[MetaAttribute],
    ) -> Result<(), ParseError> {
        if metadata.iter().all(|attribute| {
            matches!(
                attribute,
                MetaAttribute::Effect(_)
                    | MetaAttribute::MustUse(_)
                    | MetaAttribute::Deterministic
                    | MetaAttribute::Realtime
                    | MetaAttribute::Bounded(_)
                    | MetaAttribute::StackLimit(_)
                    | MetaAttribute::Capability(_)
            )
        }) {
            return Ok(());
        }
        Err(ParseError {
            code: super::ParseErrorCode::MetadataTargetNotAllowed,
            kind: super::ParseErrorKind::MetadataTargetNotAllowed,
            span: self.peek().map_or(SourceSpan::new(0, 0), |token| token.span),
        })
    }
}
