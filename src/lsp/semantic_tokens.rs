use crate::lexer::{TokenKind, scan};
use crate::target::TargetSpec;

pub(super) fn full(source: &str, target: &TargetSpec) -> serde_json::Value {
    let tokens = scan(source).0;
    let inactive = inactive_spans(source, target);
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
        let modifier = u32::from(
            inactive
                .iter()
                .any(|span| span.start <= token.span.start && token.span.end <= span.end),
        );
        data.extend([
            delta_line,
            delta_start,
            end.character - start.character,
            token_type,
            modifier,
        ]);
        previous_line = start.line;
        previous_start = start.character;
    }
    serde_json::json!({"data": data})
}

fn inactive_spans(source: &str, target: &TargetSpec) -> Vec<crate::lexer::SourceSpan> {
    let tokens = scan(source).0;
    let Ok(program) = crate::parser::parse(tokens) else { return Vec::new() };
    program
        .declarations
        .iter()
        .filter_map(|declaration| {
            let metadata = match declaration {
                crate::ast::TopLevelDecl::Verb(verb) => &verb.metadata,
                crate::ast::TopLevelDecl::ExternalVerb(verb) => &verb.metadata,
                _ => return None,
            };
            let inactive = metadata.iter().any(|attribute| {
                matches!(attribute, crate::ast::MetaAttribute::Target(selector)
                    if !target.matches_platform(selector))
            });
            inactive.then_some(match declaration {
                crate::ast::TopLevelDecl::Verb(verb) => verb.span,
                crate::ast::TopLevelDecl::ExternalVerb(verb) => verb.span,
                _ => unreachable!(),
            })
        })
        .collect()
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
        TokenKind::Integer { .. } | TokenKind::FloatLiteral { .. } => Some(1),
        TokenKind::Erg => Some(2),
        TokenKind::Abs => Some(3),
        TokenKind::Dat => Some(4),
        TokenKind::Ins => Some(5),
        _ => operator_token_type(kind),
    }
}

fn operator_token_type(kind: &TokenKind) -> Option<u32> {
    matches!(
        kind,
        TokenKind::As
            | TokenKind::LessThan
            | TokenKind::LessEquals
            | TokenKind::GreaterThan
            | TokenKind::GreaterEquals
            | TokenKind::DoubleEquals
            | TokenKind::BangEquals
            | TokenKind::Bang
            | TokenKind::Percent
            | TokenKind::AndAnd
            | TokenKind::OrOr
            | TokenKind::Ampersand
            | TokenKind::Pipe
            | TokenKind::Caret
            | TokenKind::Tilde
            | TokenKind::ShiftLeft
            | TokenKind::ShiftRight
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::PlusEquals
            | TokenKind::MinusEquals
            | TokenKind::StarEquals
            | TokenKind::SlashEquals
            | TokenKind::PercentEquals
            | TokenKind::AmpersandEquals
            | TokenKind::PipeEquals
            | TokenKind::CaretEquals
            | TokenKind::ShiftLeftEquals
            | TokenKind::ShiftRightEquals
    )
    .then_some(9)
}
