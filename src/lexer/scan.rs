use super::{LexError, Scanner, Token};

/// Tokenizes an Actus source string and returns every token and lexical error
/// found while scanning it.
pub fn scan(source: &str) -> (Vec<Token>, Vec<LexError>) {
    scan_at(source, 0)
}

pub(crate) fn scan_at(source: &str, base_offset: usize) -> (Vec<Token>, Vec<LexError>) {
    Scanner::new_at(source, base_offset).scan()
}
