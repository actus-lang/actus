use super::identifiers::is_identifier_start;
use super::{LexError, LexErrorKind, Scanner};
use crate::lexer::{SourceSpan, Token, TokenKind};

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
                SourceSpan::new(start, self.cursor),
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
            '+' if self.match_character('=') => TokenKind::PlusEquals,
            '+' => TokenKind::Plus,
            '-' if self.match_character('>') => TokenKind::Arrow,
            '-' if self.match_character('=') => TokenKind::MinusEquals,
            '-' => TokenKind::Minus,
            '*' if self.match_character('=') => TokenKind::StarEquals,
            '*' => TokenKind::Star,
            '/' if self.match_character('=') => TokenKind::SlashEquals,
            '/' => TokenKind::Slash,
            _ => return false,
        };
        self.push_simple(kind, start);
        true
    }

    fn scan_simple_token(&mut self, character: char, start: usize) -> bool {
        let kind = match character {
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            _ => return false,
        };
        self.push_simple(kind, start);
        true
    }

    fn push_simple(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token::new(kind, SourceSpan::new(start, self.cursor)));
    }
}
