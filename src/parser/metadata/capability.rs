use super::super::{ParseError, ParseErrorCode, ParseErrorKind, Parser};
use crate::ast::{Capability, MetaAttribute};
use crate::lexer::TokenKind;

impl Parser {
    pub(in crate::parser) fn reject_duplicate_capability_metadata(
        &self,
        metadata: &[MetaAttribute],
    ) -> Result<(), ParseError> {
        let Some((index, capability)) =
            metadata.iter().enumerate().find_map(|(index, attribute)| {
                if let MetaAttribute::Capability(capability) = attribute {
                    Some((index, capability))
                } else {
                    None
                }
            })
        else {
            return Ok(());
        };
        if metadata[index + 1..].iter().any(|attribute| {
            matches!(attribute, MetaAttribute::Capability(other) if other == capability)
        }) {
            return Err(ParseError {
                code: ParseErrorCode::DuplicateMetadata,
                kind: ParseErrorKind::DuplicateMetadata {
                    name: format!("capability(\"{}\")", capability.name()),
                },
                span: self
                    .peek()
                    .map(|token| token.span)
                    .unwrap_or(crate::lexer::SourceSpan::new(0, 0)),
            });
        }
        Ok(())
    }

    pub(super) fn parse_capability_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `capability`")?;
        let mut attributes = Vec::new();
        loop {
            let token = self.advance_required("capability name string")?;
            let name = match token.kind {
                TokenKind::StringLiteral(name) => name,
                found => {
                    return Err(super::syntax::unexpected_metadata_token(
                        "a capability name string",
                        found,
                        token.span,
                    ));
                }
            };
            let capability = Capability::parse(&name).ok_or(ParseError {
                code: ParseErrorCode::UnsupportedCapability,
                kind: ParseErrorKind::UnsupportedCapability { name },
                span: token.span,
            })?;
            attributes.push(MetaAttribute::Capability(capability));
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
            if self.check_simple(&TokenKind::RightParen) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "`)` after capability names")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(attributes)
    }
}
