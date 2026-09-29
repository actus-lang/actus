use crate::lexer::{TokenKind, scan};

pub(super) fn full(source: &str) -> serde_json::Value {
    let tokens = scan(source).0;
    let index = crate::lsp::position::LineIndex::new(source);
    let mut previous_line = 0;
    let mut previous_start = 0;
    let mut data = Vec::new();
    for (token_index, token) in tokens.iter().enumerate() {
        let Some(token_type) = token_type(&tokens, token_index) else { continue };
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

fn token_type(tokens: &[crate::lexer::Token], index: usize) -> Option<u32> {
    let kind = &tokens[index].kind;
    if matches!(kind, TokenKind::Pack) {
        return Some(6);
    }
    if let TokenKind::Identifier(name) = kind {
        if matches!(
            name.as_str(),
            "Int"
                | "Bool"
                | "Char"
                | "String"
                | "Buffer"
                | "Array"
                | "Arena"
                | "Option"
                | "Result"
                | "Map"
                | "Usize"
        ) {
            return Some(0);
        }
        if matches!(name.as_str(), "layout" | "fields" | "storage" | "at" | "little" | "big") {
            return Some(6);
        }
        if index > 0 && matches!(tokens[index - 1].kind, TokenKind::Pack) {
            return Some(7);
        }
        if index > 0
            && matches!(tokens[index - 1].kind, TokenKind::Erg | TokenKind::Abs)
            && index + 1 < tokens.len()
            && matches!(tokens[index + 1].kind, TokenKind::Colon)
        {
            return Some(8);
        }
    }
    match kind {
        TokenKind::IntType { .. } | TokenKind::FloatType { .. } | TokenKind::VoidType => Some(0),
        TokenKind::Integer(_) | TokenKind::FloatLiteral(_) => Some(1),
        TokenKind::Erg => Some(2),
        TokenKind::Abs => Some(3),
        TokenKind::Dat => Some(4),
        TokenKind::Ins => Some(5),
        TokenKind::As => Some(9),
        _ => None,
    }
}
