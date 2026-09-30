use crate::ast::TopLevelDecl;
use crate::formatter::{SourceComment, format_program_with_comments};
use crate::lexer::{Token, TokenKind, scan};
use crate::parser::parse;

use super::position::{LineIndex, LspRange};

pub fn format_document(source: &str) -> Option<(LspRange, String)> {
    let (tokens, errors) = scan(source);
    if !errors.is_empty() {
        return None;
    }
    let comments = source_comments(source, &tokens);
    let program = parse(tokens).ok()?;
    let range = LspRange {
        start: super::position::LspPosition { line: 0, character: 0 },
        end: LineIndex::new(source).position(source, source.len()),
    };
    Some((range, format_program_with_comments(&program, &comments)))
}

fn source_comments(source: &str, tokens: &[Token]) -> Vec<SourceComment> {
    tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::DocString(_)))
        .map(|token| SourceComment {
            span: token.span,
            text: source[token.span.start..token.span.end].to_owned(),
        })
        .collect()
}

pub fn format_range(source: &str, requested: &LspRange) -> Option<(LspRange, String)> {
    let (_, formatted) = format_document(source)?;
    let start = LineIndex::new(source).byte_offset(source, &requested.start)?;
    let end = LineIndex::new(source).byte_offset(source, &requested.end)?;
    let common_start =
        source.bytes().zip(formatted.bytes()).take_while(|(left, right)| left == right).count();
    let common_suffix = source[common_start..]
        .bytes()
        .rev()
        .zip(formatted[common_start..].bytes().rev())
        .take_while(|(left, right)| left == right)
        .count();
    let old_end = source.len().saturating_sub(common_suffix);
    let new_end = formatted.len().saturating_sub(common_suffix);
    if common_start < start || old_end > end {
        return None;
    }
    let changed_range = LspRange {
        start: LineIndex::new(source).position(source, common_start),
        end: LineIndex::new(source).position(source, old_end),
    };
    Some((changed_range, formatted[common_start..new_end].to_owned()))
}

pub fn organize_imports(source: &str) -> Option<(LspRange, String)> {
    let program = parse(scan(source).0).ok()?;
    let declarations = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Import(import) => Some((import.span, format!("import {};", import.path))),
            TopLevelDecl::OpenSibling(open) => Some((open.span, format!("open {};", open.name))),
            _ => None,
        })
        .collect::<Vec<_>>();
    if declarations.len() < 2 {
        return None;
    }
    let start = declarations.iter().map(|(span, _)| span.start).min()?;
    let end = declarations.iter().map(|(span, _)| span.end).max()?;
    let mut ordered = declarations.clone();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    if ordered.iter().zip(declarations.iter()).all(|(left, right)| left.1 == right.1) {
        return None;
    }
    if !declaration_block_is_contiguous(source, &declarations, start, end) {
        return None;
    }
    let newline = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let replacement =
        ordered.into_iter().map(|(_, declaration)| declaration).collect::<Vec<_>>().join(newline);
    Some((
        LspRange {
            start: LineIndex::new(source).position(source, start),
            end: LineIndex::new(source).position(source, end),
        },
        replacement,
    ))
}

fn declaration_block_is_contiguous(
    source: &str,
    declarations: &[(crate::lexer::SourceSpan, String)],
    start: usize,
    end: usize,
) -> bool {
    let mut spans = declarations.iter().map(|(span, _)| *span).collect::<Vec<_>>();
    spans.sort_by_key(|span| span.start);
    spans.first().is_some_and(|span| span.start == start)
        && spans.last().is_some_and(|span| span.end == end)
        && spans.windows(2).all(|pair| source[pair[0].end..pair[1].start].trim().is_empty())
}
