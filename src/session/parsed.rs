use crate::ast::Program;
use crate::lexer::{LexError, Token};
use crate::parser::ParseError;

/// Frontend result for one source revision: tokens and either a program or
/// the lexical or parse failure that stopped the frontend.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ParsedSource {
    source: String,
    tokens: Vec<Token>,
    outcome: ParseOutcome,
}

#[derive(Debug, Eq, PartialEq)]
enum ParseOutcome {
    Program(Program),
    Lex(Vec<LexError>),
    Parse(ParseError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FrontendError {
    Lex(Vec<LexError>),
    Parse(ParseError),
}

impl ParsedSource {
    /// Scans and parses `source` with spans starting at `base`.
    pub(crate) fn from_source(source: &str, base: usize) -> Self {
        let (tokens, lex_errors) = crate::lexer::scan_at(source, base);
        let outcome = if lex_errors.is_empty() {
            match crate::parser::parse(tokens.clone()) {
                Ok(program) => ParseOutcome::Program(program),
                Err(error) => ParseOutcome::Parse(error),
            }
        } else {
            ParseOutcome::Lex(lex_errors)
        };
        Self { source: source.to_owned(), tokens, outcome }
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Returns an owned program; callers normalize and annotate their copy.
    pub(crate) fn program(&self) -> Result<Program, FrontendError> {
        match &self.outcome {
            ParseOutcome::Program(program) => Ok(program.clone()),
            ParseOutcome::Lex(errors) => Err(FrontendError::Lex(errors.clone())),
            ParseOutcome::Parse(error) => Err(FrontendError::Parse(error.clone())),
        }
    }
}
