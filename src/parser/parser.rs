use crate::ast::{DispatchMode, MetaAttribute, Param, Program, Role, VerbDecl};
use crate::lexer::{SourceSpan, Token, TokenKind};
use std::collections::HashSet;

mod case;
mod cursor;
mod declarations;
mod enums;
mod expressions;
mod generics;
mod packs;
mod roles;
mod spans;
mod statements;
mod structs;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedToken { expected: String, found: TokenKind },
    UnexpectedEndOfInput { expected: String },
    DuplicateName { kind: String, name: String },
    UnknownMetadata { name: String },
    UnsupportedTargetPlatform { name: String },
    MetadataTargetNotAllowed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseErrorCode {
    UnexpectedToken,
    UnexpectedEndOfInput,
    DuplicateName,
    UnknownMetadata,
    UnsupportedTargetPlatform,
    MetadataTargetNotAllowed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub code: ParseErrorCode,
    pub kind: ParseErrorKind,
    pub span: SourceSpan,
}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    enum_names: HashSet<String>,
    case_subject: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0, enum_names: HashSet::new(), case_subject: false }
    }

    pub fn parse(mut self) -> Result<Program, ParseError> {
        let mut declarations = Vec::new();

        while !self.at_end() {
            declarations.push(self.parse_top_level_decl()?);
        }

        Ok(Program { declarations })
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

    fn parse_verb(&mut self, is_open: bool) -> Result<VerbDecl, ParseError> {
        self.parse_verb_with_metadata(is_open, Vec::new(), None)
    }

    fn parse_verb_with_metadata(
        &mut self,
        is_open: bool,
        metadata: Vec<MetaAttribute>,
        doc: Option<String>,
    ) -> Result<VerbDecl, ParseError> {
        let start = self.expect_keyword(TokenKind::Verb, "`verb`")?.span.start;
        let name_token = self.take_callable_name("verb name")?;
        let name = identifier_text(&name_token.kind);
        let generic_parameters = self.parse_generic_parameters()?;

        self.expect_simple(TokenKind::LeftParen, "`(`")?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RightParen, "`)`")?;

        let return_type = self.parse_return_type()?;

        let body = self.parse_block()?;
        let span = SourceSpan::new(start, body.span.end);

        Ok(VerbDecl {
            is_open,
            doc,
            metadata,
            name,
            generic_parameters,
            params,
            return_type,
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
            "target" => {
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
            name => Err(ParseError {
                code: ParseErrorCode::UnknownMetadata,
                kind: ParseErrorKind::UnknownMetadata { name: name.to_owned() },
                span: attribute.span,
            }),
        }
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
