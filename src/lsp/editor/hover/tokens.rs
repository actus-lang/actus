use crate::lexer::{SourceSpan, Token, TokenKind, scan};

use super::super::position::{LineIndex, LspRange};

pub(super) fn identifier_at(tokens: &[Token], offset: usize) -> Option<(String, SourceSpan)> {
    tokens.iter().find_map(|token| {
        (token.span.start <= offset
            && offset <= token.span.end
            && matches!(
                token.kind,
                TokenKind::Identifier(_)
                    | TokenKind::IntType { .. }
                    | TokenKind::FloatType { .. }
                    | TokenKind::VoidType
            ))
        .then(|| match &token.kind {
            TokenKind::Identifier(name) => (name.clone(), token.span),
            TokenKind::IntType { signed, width } => {
                (format!("{}{}", if *signed { 'i' } else { 'u' }, width), token.span)
            }
            TokenKind::FloatType { width } => (format!("f{width}"), token.span),
            TokenKind::VoidType => ("Void".to_owned(), token.span),
            _ => unreachable!(),
        })
    })
}

pub(super) fn operator_at(tokens: &[Token], offset: usize) -> Option<(&'static str, SourceSpan)> {
    tokens.iter().find_map(|token| {
        if token.span.start > offset || offset > token.span.end {
            return None;
        }
        let operator = match token.kind {
            TokenKind::As => "as",
            TokenKind::LessThan => "<",
            TokenKind::LessEquals => "<=",
            TokenKind::GreaterThan => ">",
            TokenKind::GreaterEquals => ">=",
            TokenKind::DoubleEquals => "==",
            TokenKind::BangEquals => "!=",
            TokenKind::Bang => "!",
            TokenKind::Percent => "%",
            TokenKind::AndAnd => "&&",
            TokenKind::OrOr => "||",
            TokenKind::Ampersand => "&",
            TokenKind::Pipe => "|",
            TokenKind::Caret => "^",
            TokenKind::Tilde => "~",
            TokenKind::ShiftLeft => "<<",
            TokenKind::ShiftRight => ">>",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::PlusEquals => "+=",
            TokenKind::MinusEquals => "-=",
            TokenKind::StarEquals => "*=",
            TokenKind::SlashEquals => "/=",
            TokenKind::PercentEquals => "%=",
            TokenKind::AmpersandEquals => "&=",
            TokenKind::PipeEquals => "|=",
            TokenKind::CaretEquals => "^=",
            TokenKind::ShiftLeftEquals => "<<=",
            TokenKind::ShiftRightEquals => ">>=",
            _ => return None,
        };
        Some((operator, token.span))
    })
}

pub(super) fn identifier_span(source: &str, span: SourceSpan, name: &str) -> Option<SourceSpan> {
    let (tokens, _) = scan(source.get(span.start..span.end)?);
    tokens.into_iter().find_map(|token| match token.kind {
        TokenKind::Identifier(identifier) if identifier == name => {
            Some(SourceSpan::new(span.start + token.span.start, span.start + token.span.end))
        }
        _ => None,
    })
}

pub(super) fn range(source: &str, span: SourceSpan) -> LspRange {
    let index = LineIndex::new(source);
    LspRange { start: index.position(source, span.start), end: index.position(source, span.end) }
}
