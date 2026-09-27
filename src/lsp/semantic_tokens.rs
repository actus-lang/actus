use crate::lexer::{TokenKind, scan};

pub(super) fn full(source: &str) -> serde_json::Value {
    let tokens = scan(source).0;
    let index = crate::lsp::position::LineIndex::new(source);
    let mut previous_line = 0;
    let mut previous_start = 0;
    let mut data = Vec::new();
    for token in tokens {
        let Some(token_type) = token_type(&token.kind) else { continue };
        let start = index.position(source, token.span.start);
        let end = index.position(source, token.span.end);
        let delta_line = start.line - previous_line;
        let delta_start =
            if delta_line == 0 { start.character - previous_start } else { start.character };
        data.extend([delta_line, delta_start, end.character - start.character, token_type, 0]);
        previous_line = start.line;
        previous_start = start.character;
    }
    serde_json::json!({"data": data})
}

fn token_type(kind: &TokenKind) -> Option<u32> {
    match kind {
        TokenKind::IntType { .. } | TokenKind::FloatType { .. } | TokenKind::VoidType => Some(0),
        TokenKind::Integer(_) | TokenKind::FloatLiteral(_) => Some(1),
        TokenKind::Erg => Some(2),
        TokenKind::Abs => Some(3),
        TokenKind::Dat => Some(4),
        TokenKind::Ins => Some(5),
        _ => None,
    }
}
