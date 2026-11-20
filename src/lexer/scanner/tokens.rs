use super::identifiers::is_identifier_start;
use super::{LexError, LexErrorKind, Scanner};
use crate::lexer::{Token, TokenKind};

impl<'source> Scanner<'source> {
    pub(super) fn scan_token(&mut self) {
        let start = self.cursor;
        let character = self.advance().expect("scan_token called at end of input");
        if self.scan_simple_token(character, start) {
            return;
        }
        if self.scan_operator(character, start) {
            return;
        }
        match character {
            '.' if self.match_character('.') => self.push_simple(TokenKind::DotDot, start),
            '.' => self.push_simple(TokenKind::Dot, start),
            '?' => self.push_simple(TokenKind::Question, start),
            '0' if matches!(self.peek(), Some('x' | 'X')) => self.scan_hex_integer(start),
            '"' if self.peek() == Some('"') && self.peek_next() == Some('"') => {
                self.advance();
                self.advance();
                self.scan_doc_string(start);
            }
            '"' => self.scan_string(start),
            character if is_identifier_start(character) => self.scan_identifier(start),
            character if character.is_ascii_digit() => self.scan_integer(start),
            character => self.errors.push(LexError::new(
                LexErrorKind::UnexpectedCharacter(character),
                self.span(start),
            )),
        }
    }

    fn scan_operator(&mut self, character: char, start: usize) -> bool {
        let kind = match character {
            '<' if self.peek() == Some('<') => {
                self.advance();
                if self.match_character('=') {
                    TokenKind::ShiftLeftEquals
                } else {
                    TokenKind::ShiftLeft
                }
            }
            '<' if self.match_character('=') => TokenKind::LessEquals,
            '<' => TokenKind::LessThan,
            '=' if self.match_character('>') => TokenKind::FatArrow,
            '=' if self.match_character('=') => TokenKind::DoubleEquals,
            '=' => TokenKind::Equals,
            '>' if self.peek() == Some('>') => {
                self.advance();
                if self.match_character('=') {
                    TokenKind::ShiftRightEquals
                } else {
                    TokenKind::ShiftRight
                }
            }
            '>' if self.match_character('=') => TokenKind::GreaterEquals,
            '>' => TokenKind::GreaterThan,
            '!' if self.match_character('=') => TokenKind::BangEquals,
            '!' => TokenKind::Bang,
            '%' if self.match_character('=') => TokenKind::PercentEquals,
            '%' => TokenKind::Percent,
            '&' if self.match_character('&') => TokenKind::AndAnd,
            '&' if self.match_character('=') => TokenKind::AmpersandEquals,
            '&' => TokenKind::Ampersand,
            '|' if self.match_character('|') => TokenKind::OrOr,
            '|' if self.match_character('=') => TokenKind::PipeEquals,
            '|' => TokenKind::Pipe,
            '^' if self.match_character('=') => TokenKind::CaretEquals,
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            '+' | '-' | '*' => self.arithmetic_operator(character),
            '/' if self.match_character('=') => TokenKind::SlashEquals,
            '/' => TokenKind::Slash,
            _ => return false,
        };
        self.push_simple(kind, start);
        true
    }

    /// `+`, `-` and `*` with their wrapping (`%`), saturating (`|`) and
    /// compound-assignment (`=`) forms (ADR-0115), and `->`.
    fn arithmetic_operator(&mut self, character: char) -> TokenKind {
        if character == '-' && self.match_character('>') {
            return TokenKind::Arrow;
        }
        let wrapping = self.match_character('%');
        let saturating = !wrapping && self.match_character('|');
        let compound = self.match_character('=');
        match (character, wrapping, saturating, compound) {
            ('+', true, _, false) => TokenKind::PlusPercent,
            ('+', true, _, true) => TokenKind::PlusPercentEquals,
            ('+', _, true, false) => TokenKind::PlusPipe,
            ('+', _, true, true) => TokenKind::PlusPipeEquals,
            ('+', _, _, true) => TokenKind::PlusEquals,
            ('+', ..) => TokenKind::Plus,
            ('-', true, _, false) => TokenKind::MinusPercent,
            ('-', true, _, true) => TokenKind::MinusPercentEquals,
            ('-', _, true, false) => TokenKind::MinusPipe,
            ('-', _, true, true) => TokenKind::MinusPipeEquals,
            ('-', _, _, true) => TokenKind::MinusEquals,
            ('-', ..) => TokenKind::Minus,
            (_, true, _, false) => TokenKind::StarPercent,
            (_, true, _, true) => TokenKind::StarPercentEquals,
            (_, _, true, false) => TokenKind::StarPipe,
            (_, _, true, true) => TokenKind::StarPipeEquals,
            (_, _, _, true) => TokenKind::StarEquals,
            _ => TokenKind::Star,
        }
    }

    fn scan_simple_token(&mut self, character: char, start: usize) -> bool {
        let kind = match character {
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ':' if self.match_character(':') => TokenKind::ColonColon,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            _ => return false,
        };
        self.push_simple(kind, start);
        true
    }

    fn push_simple(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token::new(kind, self.span(start)));
    }
}
