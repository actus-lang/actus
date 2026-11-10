use crate::ast::MetaAttribute;
use crate::lexer::TokenKind;
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(super) fn parse_target_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `target`")?;
        let selector_token = self.advance_required("target platform string")?;
        let selector = match selector_token.kind {
            TokenKind::StringLiteral(selector) => selector,
            found => {
                return Err(super::syntax::unexpected_metadata_token(
                    "a target platform string",
                    found,
                    selector_token.span,
                ));
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
}
