use crate::ast::{ExternalVerbDecl, ForeignAbi, TopLevelDecl};
use crate::lexer::{SourceSpan, Token, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, identifier_text};

struct ExternalVerbSignature {
    name: String,
    generic_parameters: Vec<crate::ast::GenericParam>,
    params: Vec<crate::ast::Param>,
    return_type: Option<crate::ast::ReturnType>,
    end: usize,
}

impl Parser {
    pub(super) fn parse_top_level_decl_with_doc(
        &mut self,
        mut doc: Option<String>,
    ) -> Result<Vec<TopLevelDecl>, ParseError> {
        if self.check_simple(&TokenKind::Meta) {
            let metadata = self.parse_metadata_group()?;
            doc = doc.or_else(|| self.take_doc_string_group());
            return Ok(vec![self.parse_metadata_declaration(metadata, doc)?]);
        }
        if let Some(declaration) = self.parse_keyword_declaration(doc.clone())? {
            return Ok(declaration);
        }
        self.parse_unknown_keyword()?;
        let declaration = self.parse_verb_with_metadata(false, Vec::new(), doc)?;
        Ok(vec![TopLevelDecl::Verb(declaration)])
    }

    pub(super) fn parse_metadata_group(
        &mut self,
    ) -> Result<Vec<crate::ast::MetaAttribute>, ParseError> {
        let mut metadata = Vec::new();
        while self.check_simple(&TokenKind::Meta) {
            let metadata_start = self.peek().map_or(0, |token| token.span.start);
            let parsed = self.parse_metadata()?;
            self.reject_conflicting_targets(&metadata, &parsed, metadata_start)?;
            metadata.extend(parsed);
            if metadata.iter().any(|attribute| {
                matches!(
                    attribute,
                    crate::ast::MetaAttribute::Limitless(crate::ast::LimitlessScope::File)
                )
            }) {
                break;
            }
        }
        Ok(metadata)
    }

    fn reject_conflicting_targets(
        &self,
        existing: &[crate::ast::MetaAttribute],
        parsed: &[crate::ast::MetaAttribute],
        span: usize,
    ) -> Result<(), ParseError> {
        let Some(first) = existing.iter().find_map(target_selector) else {
            return Ok(());
        };
        let Some(second) = parsed.iter().find_map(target_selector) else {
            return Ok(());
        };
        if first == second {
            return Ok(());
        }
        Err(ParseError {
            code: ParseErrorCode::ConflictingTargetPlatforms,
            kind: ParseErrorKind::ConflictingTargetPlatforms {
                first: first.to_owned(),
                second: second.to_owned(),
            },
            span: SourceSpan::new(span, span),
        })
    }

    fn parse_keyword_declaration(
        &mut self,
        doc: Option<String>,
    ) -> Result<Option<Vec<TopLevelDecl>>, ParseError> {
        if let Some(declaration) = self.parse_module_keyword(doc.clone())? {
            return Ok(Some(declaration));
        }
        if let Some(declaration) = self.parse_external_keyword(doc.clone())? {
            return Ok(Some(vec![declaration]));
        }
        if self.check_simple(&TokenKind::Const) {
            return Ok(Some(vec![TopLevelDecl::Constant(self.parse_constant(false, doc)?)]));
        }
        Ok(self.parse_type_keyword(doc)?.map(|declaration| vec![declaration]))
    }

    fn parse_module_keyword(
        &mut self,
        doc: Option<String>,
    ) -> Result<Option<Vec<TopLevelDecl>>, ParseError> {
        if self.check_simple(&TokenKind::Import) {
            return Ok(Some(self.parse_import_decl()?));
        }
        if self.check_simple(&TokenKind::Open) {
            return Ok(Some(vec![self.parse_open_top_level(doc)?]));
        }
        Ok(None)
    }

    fn parse_external_keyword(
        &mut self,
        doc: Option<String>,
    ) -> Result<Option<TopLevelDecl>, ParseError> {
        if self.check_simple(&TokenKind::Unsafe) {
            return Ok(Some(self.parse_external_declaration(true, doc)?));
        }
        if self.check_simple(&TokenKind::Extern) {
            return Ok(Some(self.parse_external_declaration(false, doc)?));
        }
        Ok(None)
    }

    fn parse_type_keyword(
        &mut self,
        doc: Option<String>,
    ) -> Result<Option<TopLevelDecl>, ParseError> {
        if self.check_simple(&TokenKind::Struct) {
            return Ok(Some(TopLevelDecl::Struct(self.parse_struct_def(false, doc)?)));
        }
        if self.check_simple(&TokenKind::Pack) {
            return Ok(Some(TopLevelDecl::Pack(self.parse_pack_decl(false, doc)?)));
        }
        if self.check_simple(&TokenKind::Serialize) {
            return Ok(Some(TopLevelDecl::Serialize(self.parse_serialize_decl(false, doc)?)));
        }
        if self.check_simple(&TokenKind::Enum) {
            return Ok(Some(TopLevelDecl::Enum(self.parse_enum_def(false, doc)?)));
        }
        if self.check_simple(&TokenKind::Role) {
            return Ok(Some(TopLevelDecl::Role(self.parse_role_decl(false, doc)?)));
        }
        if self.check_simple(&TokenKind::Perform) {
            return Ok(Some(TopLevelDecl::Perform(self.parse_perform_decl(false, doc)?)));
        }
        Ok(None)
    }

    fn parse_unknown_keyword(&self) -> Result<(), ParseError> {
        if let Some(Token { kind: TokenKind::Identifier(name), span }) = self.peek() {
            return Err(ParseError {
                code: ParseErrorCode::UnknownKeyword,
                kind: ParseErrorKind::UnknownKeyword { name: name.clone() },
                span: *span,
            });
        }
        Ok(())
    }

    pub(super) fn parse_metadata_declaration(
        &mut self,
        metadata: Vec<crate::ast::MetaAttribute>,
        doc: Option<String>,
    ) -> Result<TopLevelDecl, ParseError> {
        if self.check_simple(&TokenKind::Verb) {
            return Ok(TopLevelDecl::Verb(self.parse_verb_with_metadata(false, metadata, doc)?));
        }
        if self.check_simple(&TokenKind::Unsafe) || self.check_simple(&TokenKind::Extern) {
            return Ok(TopLevelDecl::ExternalVerb(self.parse_external_verb(
                self.check_simple(&TokenKind::Unsafe),
                false,
                metadata,
                doc,
            )?));
        }
        Err(ParseError {
            code: ParseErrorCode::MetadataTargetNotAllowed,
            kind: ParseErrorKind::MetadataTargetNotAllowed,
            span: self.peek().map(|token| token.span).unwrap_or(SourceSpan::new(0, 0)),
        })
    }

    fn parse_external_declaration(
        &mut self,
        unsafe_boundary: bool,
        doc: Option<String>,
    ) -> Result<TopLevelDecl, ParseError> {
        Ok(TopLevelDecl::ExternalVerb(self.parse_external_verb(
            unsafe_boundary,
            false,
            Vec::new(),
            doc,
        )?))
    }

    fn parse_import_decl(&mut self) -> Result<Vec<TopLevelDecl>, ParseError> {
        let start = self.expect_keyword(TokenKind::Import, "`import`")?.span.start;
        let mut items = Vec::new();
        if self.match_simple(TokenKind::LeftBrace) {
            loop {
                items.extend(self.parse_import_items()?);
                if self.match_simple(TokenKind::Comma) {
                    if self.check_simple(&TokenKind::RightBrace) {
                        break;
                    }
                    continue;
                }
                break;
            }
            self.expect_simple(TokenKind::RightBrace, "`}`")?;
        } else {
            items = self.parse_import_items()?;
        }
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(items
            .into_iter()
            .map(|(path, alias, is_open, grouped, _)| {
                TopLevelDecl::Import(crate::ast::ImportDecl {
                    path,
                    alias,
                    is_open,
                    grouped,
                    span: SourceSpan::new(start, end),
                })
            })
            .collect())
    }

    fn parse_import_items(
        &mut self,
    ) -> Result<Vec<(String, Option<String>, bool, bool, usize)>, ParseError> {
        let is_open = self.match_simple(TokenKind::Open);
        let item_start = if is_open {
            self.previous().span.start
        } else {
            self.peek().map_or(0, |token| token.span.start)
        };
        let mut segments = vec![identifier_text(&self.take_identifier("module name")?.kind)];
        while self.match_simple(TokenKind::Colon) {
            self.expect_simple(TokenKind::Colon, "`:`")?;
            if self.match_simple(TokenKind::LeftBrace) {
                let mut items = Vec::new();
                loop {
                    let mut path = segments.clone();
                    path.push(identifier_text(&self.take_identifier("imported name")?.kind));
                    let alias = self.parse_import_alias()?;
                    items.push((path.join("::"), alias, is_open, true, item_start));
                    if self.match_simple(TokenKind::Comma) {
                        if self.check_simple(&TokenKind::RightBrace) {
                            break;
                        }
                        continue;
                    }
                    break;
                }
                self.expect_simple(TokenKind::RightBrace, "`}`")?;
                return Ok(items);
            }
            segments.push(identifier_text(&self.take_identifier("module name")?.kind));
        }
        let alias = self.parse_import_alias()?;
        Ok(vec![(segments.join("::"), alias, is_open, false, item_start)])
    }

    fn parse_import_alias(&mut self) -> Result<Option<String>, ParseError> {
        if !self.match_simple(TokenKind::As) {
            return Ok(None);
        }
        Ok(Some(identifier_text(&self.take_identifier("import alias")?.kind)))
    }

    fn parse_open_top_level(&mut self, doc: Option<String>) -> Result<TopLevelDecl, ParseError> {
        self.expect_keyword(TokenKind::Open, "`open`")?;
        if self.check_identifier() {
            let start = self.previous().span.start;
            let name = identifier_text(&self.take_identifier("sibling module name")?.kind);
            let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
            return Ok(TopLevelDecl::OpenSibling(crate::ast::OpenSiblingDecl {
                doc,
                name,
                span: SourceSpan::new(start, end),
            }));
        }
        self.parse_open_declaration(doc)
    }

    fn parse_open_declaration(&mut self, doc: Option<String>) -> Result<TopLevelDecl, ParseError> {
        if self.check_simple(&TokenKind::Struct) {
            return Ok(TopLevelDecl::Struct(self.parse_struct_def(true, doc)?));
        }
        if self.check_simple(&TokenKind::Pack) {
            return Ok(TopLevelDecl::Pack(self.parse_pack_decl(true, doc)?));
        }
        if self.check_simple(&TokenKind::Serialize) {
            return Ok(TopLevelDecl::Serialize(self.parse_serialize_decl(true, doc)?));
        }
        if self.check_simple(&TokenKind::Enum) {
            return Ok(TopLevelDecl::Enum(self.parse_enum_def(true, doc)?));
        }
        if self.check_simple(&TokenKind::Role) {
            return Ok(TopLevelDecl::Role(self.parse_role_decl(true, doc)?));
        }
        if self.check_simple(&TokenKind::Perform) {
            return Ok(TopLevelDecl::Perform(self.parse_perform_decl(true, doc)?));
        }
        if self.check_simple(&TokenKind::Verb) {
            return Ok(TopLevelDecl::Verb(self.parse_verb_with_metadata(true, Vec::new(), doc)?));
        }
        if self.check_simple(&TokenKind::Const) {
            return Ok(TopLevelDecl::Constant(self.parse_constant(true, doc)?));
        }
        if self.check_simple(&TokenKind::Extern) || self.check_simple(&TokenKind::Unsafe) {
            return Ok(TopLevelDecl::ExternalVerb(self.parse_external_verb(
                self.check_simple(&TokenKind::Unsafe),
                true,
                Vec::new(),
                doc,
            )?));
        }
        Err(self.error_at_current("a declaration after `open`"))
    }

    fn parse_constant(
        &mut self,
        is_open: bool,
        doc: Option<String>,
    ) -> Result<crate::ast::ConstantDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Const, "`const`")?.span.start;
        let name = identifier_text(&self.take_identifier("constant name")?.kind);
        self.expect_simple(TokenKind::Colon, "`:`")?;
        let ty = self.parse_type_name()?;
        self.expect_simple(TokenKind::Equals, "`=`")?;
        let initializer = self.parse_expression()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(crate::ast::ConstantDecl {
            is_open,
            doc,
            name,
            ty,
            initializer,
            span: SourceSpan::new(start, end),
        })
    }

    fn parse_external_verb(
        &mut self,
        unsafe_boundary: bool,
        is_open: bool,
        metadata: Vec<crate::ast::MetaAttribute>,
        doc: Option<String>,
    ) -> Result<ExternalVerbDecl, ParseError> {
        let (start, abi) = self.parse_external_prefix(unsafe_boundary)?;
        self.expect_keyword(TokenKind::Verb, "`verb`")?;
        let signature = self.parse_external_signature()?;
        let span = SourceSpan::new(start, signature.end);
        let contract = self.parse_contract(doc.as_deref(), span)?;
        Ok(ExternalVerbDecl {
            is_open,
            doc,
            contract,
            unsafe_boundary,
            module_import: false,
            abi,
            metadata,
            name: signature.name,
            native_symbol: None,
            generic_parameters: signature.generic_parameters,
            params: signature.params,
            return_type: signature.return_type,
            span,
        })
    }

    fn parse_external_prefix(
        &mut self,
        unsafe_boundary: bool,
    ) -> Result<(usize, ForeignAbi), ParseError> {
        let start = if unsafe_boundary {
            let start = self.expect_keyword(TokenKind::Unsafe, "`unsafe`")?.span.start;
            self.expect_keyword(TokenKind::Extern, "`extern`")?;
            start
        } else {
            self.expect_keyword(TokenKind::Extern, "`extern`")?.span.start
        };
        let abi_token = self.advance_required("ABI string")?;
        let abi = match &abi_token.kind {
            TokenKind::StringLiteral(abi) if abi == "C" => ForeignAbi::C,
            _ => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "supported ABI string (currently `\"C\"`)".to_owned(),
                        found: abi_token.kind.clone(),
                    },
                    span: abi_token.span,
                });
            }
        };
        Ok((start, abi))
    }

    fn parse_external_signature(&mut self) -> Result<ExternalVerbSignature, ParseError> {
        let name_token = self.take_identifier("external verb name")?;
        let name = identifier_text(&name_token.kind);
        let generic_parameters = self.parse_generic_parameters()?;
        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RightParen, "`)`")?;
        let return_type = self.parse_return_type()?;
        let end = self.expect_simple(TokenKind::Semicolon, "`;`")?.span.end;
        Ok(ExternalVerbSignature { name, generic_parameters, params, return_type, end })
    }
}

fn target_selector(attribute: &crate::ast::MetaAttribute) -> Option<&str> {
    match attribute {
        crate::ast::MetaAttribute::Target(selector) => Some(selector),
        crate::ast::MetaAttribute::Test => None,
        crate::ast::MetaAttribute::Limitless(_) => None,
    }
}
