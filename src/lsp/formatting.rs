use crate::formatter::format_program;
use crate::lexer::scan;
use crate::parser::parse;

use super::position::{LineIndex, LspRange};

pub fn format_document(source: &str) -> Option<(LspRange, String)> {
    let program = parse(scan(source).0).ok()?;
    let range = LspRange {
        start: super::position::LspPosition { line: 0, character: 0 },
        end: LineIndex::new(source).position(source, source.len()),
    };
    Some((range, format_program(&program)))
}
