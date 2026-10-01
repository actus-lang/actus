use super::{LexError, LexErrorKind, Scanner};
use crate::lexer::{SourceSpan, Token, TokenKind};

impl<'source> Scanner<'source> {
    pub(super) fn scan_identifier(&mut self, start: usize) {
        while self.peek().is_some_and(is_identifier_continue) {
            self.advance();
        }
        let text = &self.source[start..self.cursor];
        if let Some(kind) = integer_type_kind(text) {
            self.tokens.push(Token::new(kind, SourceSpan::new(start, self.cursor)));
            return;
        }
        if is_invalid_integer_type(text) {
            self.errors.push(LexError::new(
                LexErrorKind::InvalidIntegerType(text.to_owned()),
                SourceSpan::new(start, self.cursor),
            ));
        }
        self.tokens
            .push(Token::new(keyword_or_identifier(text), SourceSpan::new(start, self.cursor)));
    }
}

pub(super) fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn keyword_or_identifier(text: &str) -> TokenKind {
    match text {
        "verb" => TokenKind::Verb,
        "extern" => TokenKind::Extern,
        "unsafe" => TokenKind::Unsafe,
        "erg" => TokenKind::Erg,
        "abs" => TokenKind::Abs,
        "dat" => TokenKind::Dat,
        "ins" => TokenKind::Ins,
        "ref" => TokenKind::Ref,
        "drop" => TokenKind::Drop,
        "return" => TokenKind::Return,
        "loop" => TokenKind::Loop,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "struct" => TokenKind::Struct,
        "pack" => TokenKind::Pack,
        "enum" => TokenKind::Enum,
        "role" => TokenKind::Role,
        "perform" => TokenKind::Perform,
        "dynamic" => TokenKind::Dynamic,
        "meta" => TokenKind::Meta,
        "open" => TokenKind::Open,
        "import" => TokenKind::Import,
        "for" => TokenKind::For,
        "case" => TokenKind::Case,
        "as" => TokenKind::As,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "_" => TokenKind::Underscore,
        "f32" => TokenKind::FloatType { width: 32 },
        "f64" => TokenKind::FloatType { width: 64 },
        "Void" => TokenKind::VoidType,
        _ => TokenKind::Identifier(text.to_owned()),
    }
}

fn integer_type_kind(text: &str) -> Option<TokenKind> {
    let signed = match text.as_bytes().first().copied() {
        Some(b'u') => false,
        Some(b'i') => true,
        _ => return None,
    };
    let digits = text.get(1..)?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let width = digits.parse::<u16>().ok()?;
    (1..=128).contains(&width).then_some(TokenKind::IntType { signed, width: width as u8 })
}

fn is_invalid_integer_type(text: &str) -> bool {
    let Some(prefix) = text.as_bytes().first() else { return false };
    matches!(prefix, b'u' | b'i')
        && text.len() > 1
        && text[1..].bytes().all(|byte| byte.is_ascii_digit())
}
