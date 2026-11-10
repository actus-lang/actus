use crate::ast::{MetaAttribute, TopLevelDecl};
use crate::lexer::{SourceSpan, TokenKind};
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(in crate::parser) fn file_metadata_error(&self) -> ParseError {
        ParseError {
            code: ParseErrorCode::MetadataFileScopeNotAllowed,
            kind: ParseErrorKind::MetadataFileScopeNotAllowed,
            span: self.peek().map(|token| token.span).unwrap_or(SourceSpan::new(0, 0)),
        }
    }

    pub(in crate::parser) fn parse_metadata_declaration(
        &mut self,
        metadata: Vec<MetaAttribute>,
        doc: Option<String>,
    ) -> Result<TopLevelDecl, ParseError> {
        self.validate_metadata_target(&metadata)?;
        let is_open = self.match_simple(TokenKind::Open);
        ensure_type_metadata_target(self, &metadata)?;
        if self.check_simple(&TokenKind::Verb) {
            let verb = self.parse_verb_with_metadata(is_open, metadata.clone(), doc)?;
            ensure_must_use_return(&metadata, verb.return_type.is_some(), verb.span)?;
            return Ok(TopLevelDecl::Verb(verb));
        }
        if self.check_simple(&TokenKind::Struct) {
            ensure_repr_type_metadata(&metadata, self.metadata_span())?;
            return Ok(TopLevelDecl::Struct(
                self.parse_struct_def_with_metadata(is_open, metadata, doc)?,
            ));
        }
        if self.check_simple(&TokenKind::Enum) {
            ensure_repr_type_metadata(&metadata, self.metadata_span())?;
            return Ok(TopLevelDecl::Enum(
                self.parse_enum_def_with_metadata(is_open, metadata, doc)?,
            ));
        }
        if self.check_simple(&TokenKind::Unsafe) || self.check_simple(&TokenKind::Extern) {
            let verb = self.parse_external_verb(
                self.check_simple(&TokenKind::Unsafe),
                is_open,
                metadata,
                doc,
            )?;
            ensure_must_use_return(&verb.metadata, verb.return_type.is_some(), verb.span)?;
            return Ok(TopLevelDecl::ExternalVerb(verb));
        }
        Err(ParseError {
            code: ParseErrorCode::MetadataTargetNotAllowed,
            kind: ParseErrorKind::MetadataTargetNotAllowed,
            span: self.metadata_span(),
        })
    }

    fn validate_metadata_target(&self, metadata: &[MetaAttribute]) -> Result<(), ParseError> {
        let span = self.metadata_span();
        if has_fault(metadata) && !has_test(metadata) {
            return Err(ParseError {
                code: ParseErrorCode::FaultRequiresTest,
                kind: ParseErrorKind::FaultRequiresTest,
                span,
            });
        }
        if has_test_or_fault(metadata)
            && (self.check_simple(&TokenKind::Unsafe) || self.check_simple(&TokenKind::Extern))
        {
            return Err(ParseError {
                code: ParseErrorCode::TestMetadataRequiresVerb,
                kind: ParseErrorKind::TestMetadataRequiresVerb,
                span,
            });
        }
        Ok(())
    }

    pub(super) fn metadata_span(&self) -> SourceSpan {
        self.peek().map(|token| token.span).unwrap_or(SourceSpan::new(0, 0))
    }
}

fn ensure_type_metadata_target(
    parser: &Parser,
    metadata: &[MetaAttribute],
) -> Result<(), ParseError> {
    let has_volatile =
        metadata.iter().any(|attribute| matches!(attribute, MetaAttribute::Volatile));
    let has_zero_on_drop =
        metadata.iter().any(|attribute| matches!(attribute, MetaAttribute::ZeroOnDrop));
    let is_type = parser.check_simple(&TokenKind::Struct) || parser.check_simple(&TokenKind::Enum);
    if ((has_volatile || has_zero_on_drop) && !parser.check_simple(&TokenKind::Struct))
        || (metadata.iter().any(|attribute| {
            matches!(
                attribute,
                MetaAttribute::Repr(_) | MetaAttribute::Volatile | MetaAttribute::ZeroOnDrop
            )
        }) && !is_type)
    {
        return Err(ParseError {
            code: ParseErrorCode::MetadataTargetNotAllowed,
            kind: ParseErrorKind::MetadataTargetNotAllowed,
            span: parser.metadata_span(),
        });
    }
    Ok(())
}

fn ensure_repr_type_metadata(
    metadata: &[MetaAttribute],
    span: SourceSpan,
) -> Result<(), ParseError> {
    if metadata.iter().all(|attribute| {
        matches!(
            attribute,
            MetaAttribute::Repr(_) | MetaAttribute::Volatile | MetaAttribute::ZeroOnDrop
        )
    }) {
        return Ok(());
    }
    Err(ParseError {
        code: ParseErrorCode::MetadataTargetNotAllowed,
        kind: ParseErrorKind::MetadataTargetNotAllowed,
        span,
    })
}

fn has_test(metadata: &[MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, MetaAttribute::Test))
}

fn has_fault(metadata: &[MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, MetaAttribute::Fault(_)))
}

fn has_test_or_fault(metadata: &[MetaAttribute]) -> bool {
    metadata
        .iter()
        .any(|attribute| matches!(attribute, MetaAttribute::Test | MetaAttribute::Fault(_)))
}

fn ensure_must_use_return(
    metadata: &[MetaAttribute],
    has_return: bool,
    span: SourceSpan,
) -> Result<(), ParseError> {
    if metadata.iter().any(|attribute| matches!(attribute, MetaAttribute::MustUse(_)))
        && !has_return
    {
        return Err(ParseError {
            code: ParseErrorCode::MustUseRequiresReturn,
            kind: ParseErrorKind::MustUseRequiresReturn,
            span,
        });
    }
    Ok(())
}
