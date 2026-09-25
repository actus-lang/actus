use super::token::{SourceSpan, Token, TokenKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LexErrorKind {
    UnexpectedCharacter(char),
    UnterminatedString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: SourceSpan,
}

impl LexError {
    const fn new(kind: LexErrorKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}

pub struct Scanner<'source> {
    source: &'source str,
    cursor: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'source> Scanner<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source, cursor: 0, tokens: Vec::new(), errors: Vec::new() }
    }

    pub fn scan(mut self) -> (Vec<Token>, Vec<LexError>) {
        while !self.is_at_end() {
            self.skip_whitespace_and_comments();

            if self.is_at_end() {
                break;
            }

            self.scan_token();
        }

        let end = self.cursor;
        self.tokens.push(Token::new(TokenKind::Eof, SourceSpan::new(end, end)));

        (self.tokens, self.errors)
    }

    fn scan_token(&mut self) {
        let start = self.cursor;
        let character = self.advance().expect("scan_token called at end of input");

        match character {
            '{' => self.push_simple(TokenKind::LeftBrace, start),
            '}' => self.push_simple(TokenKind::RightBrace, start),
            '(' => self.push_simple(TokenKind::LeftParen, start),
            ')' => self.push_simple(TokenKind::RightParen, start),
            '[' => self.push_simple(TokenKind::LeftBracket, start),
            ']' => self.push_simple(TokenKind::RightBracket, start),
            ':' => self.push_simple(TokenKind::Colon, start),
            ',' => self.push_simple(TokenKind::Comma, start),
            ';' => self.push_simple(TokenKind::Semicolon, start),
            '<' if self.match_character('=') => self.push_simple(TokenKind::LessEquals, start),
            '<' => self.push_simple(TokenKind::LessThan, start),
            '=' if self.match_character('>') => self.push_simple(TokenKind::FatArrow, start),
            '=' if self.match_character('=') => self.push_simple(TokenKind::DoubleEquals, start),
            '=' => self.push_simple(TokenKind::Equals, start),
            '>' if self.match_character('=') => self.push_simple(TokenKind::GreaterEquals, start),
            '>' => self.push_simple(TokenKind::GreaterThan, start),
            '!' if self.match_character('=') => self.push_simple(TokenKind::BangEquals, start),
            '!' => self.push_simple(TokenKind::Bang, start),
            '+' => self.push_simple(TokenKind::Plus, start),
            '-' if self.match_character('>') => {
                self.push_simple(TokenKind::Arrow, start);
            }
            '-' => self.push_simple(TokenKind::Minus, start),
            '*' => self.push_simple(TokenKind::Star, start),
            '/' => self.push_simple(TokenKind::Slash, start),
            '.' => self.push_simple(TokenKind::Dot, start),
            '"' => self.scan_string(start),
            character if is_identifier_start(character) => self.scan_identifier(start),
            character if character.is_ascii_digit() => self.scan_integer(start),
            character => self.errors.push(LexError::new(
                LexErrorKind::UnexpectedCharacter(character),
                SourceSpan::new(start, self.cursor),
            )),
        }
    }

    fn scan_identifier(&mut self, start: usize) {
        while self.peek().is_some_and(is_identifier_continue) {
            self.advance();
        }

        let text = &self.source[start..self.cursor];
        let kind = match text {
            "verb" => TokenKind::Verb,
            "extern" => TokenKind::Extern,
            "unsafe" => TokenKind::Unsafe,
            "erg" => TokenKind::Erg,
            "abs" => TokenKind::Abs,
            "dat" => TokenKind::Dat,
            "ref" => TokenKind::Ref,
            "drop" => TokenKind::Drop,
            "return" => TokenKind::Return,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "role" => TokenKind::Role,
            "perform" => TokenKind::Perform,
            "dynamic" => TokenKind::Dynamic,
            "meta" => TokenKind::Meta,
            "open" => TokenKind::Open,
            "import" => TokenKind::Import,
            "for" => TokenKind::For,
            "case" => TokenKind::Case,
            "if" => TokenKind::If,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "_" => TokenKind::Underscore,
            _ => TokenKind::Identifier(text.to_owned()),
        };

        self.tokens.push(Token::new(kind, SourceSpan::new(start, self.cursor)));
    }

    fn scan_integer(&mut self, start: usize) {
        while self.peek().is_some_and(|character| character.is_ascii_digit()) {
            self.advance();
        }

        if self.peek() == Some('.')
            && self.peek_next().is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
            while self.peek().is_some_and(|character| character.is_ascii_digit()) {
                self.advance();
            }
            let text = &self.source[start..self.cursor];
            self.tokens.push(Token::new(
                TokenKind::FloatLiteral(text.to_owned()),
                SourceSpan::new(start, self.cursor),
            ));
            return;
        }

        let text = &self.source[start..self.cursor];
        self.tokens.push(Token::new(
            TokenKind::Integer(text.to_owned()),
            SourceSpan::new(start, self.cursor),
        ));
    }

    fn scan_string(&mut self, start: usize) {
        let content_start = self.cursor;

        while let Some(character) = self.peek() {
            match character {
                '"' => {
                    let content = self.source[content_start..self.cursor].to_owned();
                    self.advance();
                    self.tokens.push(Token::new(
                        TokenKind::StringLiteral(content),
                        SourceSpan::new(start, self.cursor),
                    ));
                    return;
                }
                '\\' => {
                    self.advance();
                    if !self.is_at_end() {
                        self.advance();
                    }
                }
                '\n' | '\r' => break,
                _ => {
                    self.advance();
                }
            }
        }

        self.errors.push(LexError::new(
            LexErrorKind::UnterminatedString,
            SourceSpan::new(start, self.cursor),
        ));
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.advance();
            }

            if self.peek() != Some('/') || self.peek_next() != Some('/') {
                return;
            }

            while self.peek().is_some_and(|character| character != '\n') {
                self.advance();
            }
        }
    }

    fn push_simple(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token::new(kind, SourceSpan::new(start, self.cursor)));
    }

    fn match_character(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor += character.len_utf8();
        Some(character)
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn peek_next(&self) -> Option<char> {
        self.source[self.cursor..].chars().nth(1)
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.source.len()
    }
}

fn is_identifier_start(character: char) -> bool {
    character == '_' || character.is_ascii_alphabetic()
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}
