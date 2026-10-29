use super::super::{ParseError, ParseErrorCode, ParseErrorKind};
use crate::ast::TestFault;
use crate::lexer::{SourceSpan, TokenKind};

pub(super) fn unexpected_metadata_token(
    expected: &str,
    found: TokenKind,
    span: SourceSpan,
) -> ParseError {
    ParseError {
        code: ParseErrorCode::UnexpectedToken,
        kind: ParseErrorKind::UnexpectedToken { expected: expected.to_owned(), found },
        span,
    }
}

pub(super) fn parse_fault_name(token: crate::lexer::Token) -> Result<TestFault, ParseError> {
    match token.kind {
        TokenKind::StringLiteral(name) => TestFault::parse(&name).ok_or(ParseError {
            code: ParseErrorCode::UnknownMetadata,
            kind: ParseErrorKind::UnknownMetadata { name },
            span: token.span,
        }),
        found => {
            Err(unexpected_metadata_token("a publication fault name string", found, token.span))
        }
    }
}
