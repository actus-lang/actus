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
    Ins,
    Ref,
    Drop,
    Return,
    Loop,
    Break,
    Continue,
    Struct,
    Pack,
    Enum,
    Role,
    Perform,
    Const,
    Dynamic,
    Meta,
    Open,
    Import,
    For,
    Repeat,
    In,
    Case,
    As,
    If,
    Else,
    True,
    False,
    Underscore,
    IntType { signed: bool, width: u8 },
    FloatType { width: u8 },
    VoidType,
    Identifier(String),
    Integer { value: String, suffix: Option<String> },
    FloatLiteral { value: String, suffix: Option<String> },
    StringLiteral(String),
    DocString(String),
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
    PlusEquals,
    MinusEquals,
    StarEquals,
    SlashEquals,
    PercentEquals,
    AmpersandEquals,
    PipeEquals,
    CaretEquals,
    ShiftLeftEquals,
    ShiftRightEquals,
    LessThan,
    LessEquals,
    GreaterThan,
    GreaterEquals,
    DoubleEquals,
    BangEquals,
    Bang,
    Percent,
    AndAnd,
    OrOr,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    ShiftLeft,
    ShiftRight,
    Plus,
    Minus,
    Star,
    Slash,
    Arrow,
    FatArrow,
    Dot,
    DotDot,
    Question,
    Eof,
}
