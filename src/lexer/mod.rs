mod scanner;
mod token;

pub use scanner::{LexError, LexErrorKind, Scanner};
pub use token::{SourceSpan, Token, TokenKind};

/// Tokenizes an Actus source string and returns every token and lexical error
/// found while scanning it.
pub fn scan(source: &str) -> (Vec<Token>, Vec<LexError>) {
    Scanner::new(source).scan()
}
