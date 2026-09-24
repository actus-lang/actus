/// A half-open byte range in the original source string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

impl SourceSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

impl Token {
    pub const fn new(kind: TokenKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Verb,
    Extern,
    Unsafe,
    Erg,
    Abs,
    Dat,
    Ref,
    Drop,
    Return,
    Loop,
    Break,
    Continue,
    Struct,
    Enum,
    Role,
    Perform,
    Dynamic,
    Open,
    Import,
    For,
    Case,
    If,
    True,
    False,
    Underscore,
    Identifier(String),
    Integer(String),
    FloatLiteral(String),
    StringLiteral(String),
    LeftBrace,
    RightBrace,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Colon,
    Comma,
    Semicolon,
    Equals,
    LessThan,
    LessEquals,
    GreaterThan,
    GreaterEquals,
    DoubleEquals,
    BangEquals,
    Bang,
    Plus,
    Minus,
    Star,
    Slash,
    Arrow,
    FatArrow,
    Dot,
    Eof,
}
