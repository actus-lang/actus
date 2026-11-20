//! Source comments the formatter keeps: `"""` documentation strings, which
//! the lexer emits as tokens, and `#` line comments, which it skips as trivia
//! between tokens.

use crate::lexer::{SourceSpan, Token, TokenKind};

/// A documentation string or `#` comment and where it stood in the source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceComment {
    pub span: SourceSpan,
    pub text: String,
    /// A `#` comment that follows code on its source line.
    pub trailing: bool,
}

/// Every documentation string and `#` comment of `source`, in source order.
/// `tokens` are the lexer's tokens for `source`, scanned from offset 0.
pub fn source_comments(source: &str, tokens: &[Token]) -> Vec<SourceComment> {
    let mut comments = Vec::new();
    let mut gap_start = 0;
    for token in tokens {
        push_line_comments(source, gap_start, token.span.start, &mut comments);
        if matches!(token.kind, TokenKind::DocString(_)) {
            comments.push(SourceComment {
                span: token.span,
                text: source[token.span.start..token.span.end].to_owned(),
                trailing: false,
            });
        }
        gap_start = token.span.end;
    }
    push_line_comments(source, gap_start, source.len(), &mut comments);
    comments
}

/// The `#` comments between two tokens. The lexer leaves only whitespace and
/// comments there, so every `#` in the gap starts a comment.
fn push_line_comments(source: &str, start: usize, end: usize, comments: &mut Vec<SourceComment>) {
    let mut cursor = start;
    while let Some(offset) = source[cursor..end].find('#') {
        let comment_start = cursor + offset;
        let comment_end =
            source[comment_start..end].find('\n').map_or(end, |newline| comment_start + newline);
        let line_start = source[..comment_start].rfind('\n').map_or(0, |newline| newline + 1);
        comments.push(SourceComment {
            span: SourceSpan::new(comment_start, comment_end),
            text: source[comment_start..comment_end].trim_end().to_owned(),
            trailing: !source[line_start..comment_start].trim().is_empty(),
        });
        cursor = comment_end;
    }
}
