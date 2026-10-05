use crate::ast::{DispatchMode, LimitlessScope, MetaAttribute, Param, Program, Role, VerbDecl};
use crate::lexer::{SourceSpan, Token, TokenKind};
use std::collections::HashSet;

mod case;
mod conditional;
mod contracts;
mod cursor;
mod declarations;
mod enums;
mod expressions;
mod generics;
mod packs;
mod roles;
mod serialization;
mod spans;
mod statements;
mod structs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedToken { expected: String, found: TokenKind },
    UnexpectedEndOfInput { expected: String },
    DuplicateName { kind: String, name: String },
    UnknownKeyword { name: String },
    UnknownMetadata { name: String },
    UnsupportedTargetPlatform { name: String },
    ConflictingTargetPlatforms { first: String, second: String },
    MetadataTargetNotAllowed,
    MetadataFileScopeNotAllowed,
    UnsupportedLimitlessScope { name: String },
    DuplicateMetadata { name: String },
    MalformedVerbContract { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseErrorCode {
    UnexpectedToken,
    UnexpectedEndOfInput,
    DuplicateName,
    UnknownKeyword,
    UnknownMetadata,
    UnsupportedTargetPlatform,
    ConflictingTargetPlatforms,
    MetadataTargetNotAllowed,
    MetadataFileScopeNotAllowed,
    UnsupportedLimitlessScope,
    DuplicateMetadata,
    MalformedVerbContract,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub kind: ParseErrorKind,
    pub span: SourceSpan,
}

fn contract_error_message(error: contracts::ContractParseError) -> String {
    match error {
        contracts::ContractParseError::ContentBeforePurpose => {
            "contract content must begin with a named section".to_owned()
        }
        contracts::ContractParseError::DuplicateSection(name) => {
            format!("contract section `{name}` is declared more than once")
        }
        contracts::ContractParseError::EmptyContract => {
            "a structured verb contract must contain at least one named section".to_owned()
        }
        contracts::ContractParseError::MissingSectionName => {
            "contract section name must not be empty".to_owned()
        }
        contracts::ContractParseError::UnknownSection(name) => {
            format!("unknown contract section `{name}`")
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    enum_names: HashSet<String>,
    case_subject: bool,
}

struct VerbSignature {
    name: String,
    generic_parameters: Vec<crate::ast::GenericParam>,
    params: Vec<Param>,
    return_type: Option<crate::ast::ReturnType>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0, enum_names: HashSet::new(), case_subject: false }
    }

    pub(super) fn parse_contract(
        &self,
        documentation: Option<&str>,
        span: SourceSpan,
    ) -> Result<Option<crate::ast::VerbContract>, ParseError> {
        contracts::parse_contract(documentation, span).map_err(|error| ParseError {
            code: ParseErrorCode::MalformedVerbContract,
            kind: ParseErrorKind::MalformedVerbContract { reason: contract_error_message(error) },
            span,
        })
    }

    fn parse_verb_signature(&mut self) -> Result<VerbSignature, ParseError> {
        let name_token = self.take_callable_name("verb name")?;
        let name = identifier_text(&name_token.kind);
        let generic_parameters = self.parse_generic_parameters()?;
        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RightParen, "`)`")?;
        let return_type = self.parse_return_type()?;
        Ok(VerbSignature { name, generic_parameters, params, return_type })
    }
    pub fn parse(mut self) -> Result<Program, ParseError> {
        let mut file_metadata = Vec::new();
        let mut declarations = Vec::new();

        while !self.at_end() {
            let doc = self.take_doc_string_group();
            if self.check_simple(&TokenKind::Meta) {
                let metadata = self.parse_metadata_group()?;
                if metadata.iter().any(|attribute| {
                    matches!(attribute, MetaAttribute::Limitless(LimitlessScope::File))
                }) {
                    if !declarations.is_empty()
                        || !file_metadata.is_empty()
                        || doc.is_some()
                        || metadata.len() != 1
                    {
                        return Err(self.file_metadata_error());
                    }
                    file_metadata.push(LimitlessScope::File);
                    continue;
                }
                let doc = doc.or_else(|| self.take_doc_string_group());
                self.reject_duplicate_metadata(&metadata)?;
                declarations.push(self.parse_metadata_declaration(metadata, doc)?);
                continue;
            }
            declarations.push(self.parse_top_level_decl_with_doc(doc)?);
        }

        Ok(Program { file_metadata, declarations })
    }

    fn reject_duplicate_metadata(&self, metadata: &[MetaAttribute]) -> Result<(), ParseError> {
        let Some((index, scope)) = metadata.iter().enumerate().find_map(|(index, attribute)| {
            matches!(attribute, MetaAttribute::Limitless(LimitlessScope::Verb))
                .then_some((index, "limitless"))
        }) else {
            return Ok(());
        };
        if metadata[index + 1..]
            .iter()
            .any(|attribute| matches!(attribute, MetaAttribute::Limitless(LimitlessScope::Verb)))
        {
            return Err(ParseError {
                code: ParseErrorCode::DuplicateMetadata,
                kind: ParseErrorKind::DuplicateMetadata { name: scope.to_owned() },
                span: self.peek().map(|token| token.span).unwrap_or(SourceSpan::new(0, 0)),
            });
        }
        Ok(())
    }

    fn file_metadata_error(&self) -> ParseError {
        ParseError {
            code: ParseErrorCode::MetadataFileScopeNotAllowed,
            kind: ParseErrorKind::MetadataFileScopeNotAllowed,
            span: self.peek().map(|token| token.span).unwrap_or(SourceSpan::new(0, 0)),
        }
    }

    fn take_doc_string(&mut self) -> Option<String> {
        let Some(Token { kind: TokenKind::DocString(doc), .. }) = self.peek().cloned() else {
            return None;
        };
        self.cursor += 1;
        Some(doc)
    }

    fn take_doc_string_group(&mut self) -> Option<String> {
        let mut documents = Vec::new();
        while let Some(document) = self.take_doc_string() {
            documents.push(document);
        }
        (!documents.is_empty()).then(|| documents.join("\n\n"))
    }

    fn skip_doc_strings(&mut self) {
        while self.peek().is_some_and(|token| matches!(token.kind, TokenKind::DocString(_))) {
            self.cursor += 1;
        }
    }

    fn parse_verb_with_metadata(
        &mut self,
        is_open: bool,
        metadata: Vec<MetaAttribute>,
        doc: Option<String>,
    ) -> Result<VerbDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Verb, "`verb`")?.span.start;
        let signature = self.parse_verb_signature()?;
        let body = self.parse_block()?;
        let span = SourceSpan::new(start, body.span.end);
        let contract = self.parse_contract(doc.as_deref(), span)?;

        Ok(VerbDecl {
            is_open,
            doc,
            contract,
            metadata,
            name: signature.name,
            generic_parameters: signature.generic_parameters,
            params: signature.params,
            return_type: signature.return_type,
            body,
            span,
        })
    }

    fn parse_return_type(&mut self) -> Result<Option<crate::ast::ReturnType>, ParseError> {
        if !self.match_simple(TokenKind::Arrow) {
            return Ok(None);
        }
        let start = self.previous().span.start;
        let access = if self.match_simple(TokenKind::Abs) {
            crate::ast::ReturnAccess::Abs
        } else {
            crate::ast::ReturnAccess::Owned
        };
        let ty = self.parse_type_name()?;
        let span = SourceSpan::new(start, ty.span.end);
        Ok(Some(crate::ast::ReturnType { access, ty, span }))
    }

    fn parse_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_keyword(TokenKind::Meta, "`meta`")?;
        let attribute = self.take_identifier("metadata name")?;
        match identifier_text(&attribute.kind).as_str() {
            "test" => {
                let _ = self.match_simple(TokenKind::Semicolon);
                Ok(vec![MetaAttribute::Test])
            }
            "target" => self.parse_target_metadata(),
            "limitless" => self.parse_limitless_metadata(),
            name => Err(ParseError {
                code: ParseErrorCode::UnknownMetadata,
                kind: ParseErrorKind::UnknownMetadata { name: name.to_owned() },
                span: attribute.span,
            }),
        }
    }

    fn parse_limitless_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `limitless`")?;
        let scope_token = self.advance_required("limitless scope string")?;
        let scope = match scope_token.kind {
            TokenKind::StringLiteral(scope) => match scope.as_str() {
                "verb" => LimitlessScope::Verb,
                "file" => LimitlessScope::File,
                _ => {
                    return Err(ParseError {
                        code: ParseErrorCode::UnsupportedLimitlessScope,
                        kind: ParseErrorKind::UnsupportedLimitlessScope { name: scope },
                        span: scope_token.span,
                    });
                }
            },
            found => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "`verb` or `file` scope string".to_owned(),
                        found,
                    },
                    span: scope_token.span,
                });
            }
        };
        self.expect_simple(TokenKind::RightParen, "`)` after limitless scope")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Limitless(scope)])
    }

    fn parse_target_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `target`")?;
        let selector_token = self.advance_required("target platform string")?;
        let selector = match selector_token.kind {
            TokenKind::StringLiteral(selector) => selector,
            found => {
                return Err(ParseError {
                    code: ParseErrorCode::UnexpectedToken,
                    kind: ParseErrorKind::UnexpectedToken {
                        expected: "a target platform string".to_owned(),
                        found,
                    },
                    span: selector_token.span,
                });
            }
        };
        if !matches!(selector.as_str(), "unix" | "posix" | "windows") {
            return Err(ParseError {
                code: ParseErrorCode::UnsupportedTargetPlatform,
                kind: ParseErrorKind::UnsupportedTargetPlatform { name: selector },
                span: selector_token.span,
            });
        }
        self.expect_simple(TokenKind::RightParen, "`)` after target platform")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Target(selector)])
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();

        if self.check_simple(&TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            params.push(self.parse_param()?);
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let role_token = self.advance_required("parameter role")?;
        let role = parameter_role_from_token(&role_token.kind).ok_or_else(|| ParseError {
            code: ParseErrorCode::UnexpectedToken,
            kind: ParseErrorKind::UnexpectedToken {
                expected: "parameter role (`erg`, `abs`, `dat`, or `ins`)".to_owned(),
                found: role_token.kind.clone(),
            },
            span: role_token.span,
        })?;

        let name_token = self.take_identifier("parameter name")?;
        let name = identifier_text(&name_token.kind);
        self.expect_simple(TokenKind::Colon, "`:`")?;
        let dispatch = if self.match_simple(TokenKind::Dynamic) {
            DispatchMode::Dynamic
        } else {
            DispatchMode::Static
        };
        let ty = self.parse_type_name()?;

        Ok(Param {
            role,
            name,
            dispatch,
            span: SourceSpan::new(role_token.span.start, ty.span.end),
            ty,
        })
    }
}

fn identifier_text(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Identifier(name) => name.clone(),
        TokenKind::Drop => "drop".to_owned(),
        _ => unreachable!("identifier_text called with a non-identifier token"),
    }
}

fn parameter_role_from_token(kind: &TokenKind) -> Option<Role> {
    match kind {
        TokenKind::Erg => Some(Role::Erg),
        TokenKind::Abs => Some(Role::Abs),
        TokenKind::Dat => Some(Role::Dat),
        TokenKind::Ins => Some(Role::Ins),
        _ => None,
    }
}

pub(super) use spans::expression_span;
