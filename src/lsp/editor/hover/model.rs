use crate::lexer::SourceSpan;

#[derive(Clone, Debug)]
pub(in crate::lsp) struct HoverInfo {
    pub contents: String,
    pub range: crate::lsp::workspace::position::LspRange,
}

#[derive(Clone, Debug)]
pub(super) struct SymbolInfo {
    pub(super) signature: String,
    pub(super) span: SourceSpan,
    pub(super) documentation: Option<String>,
}
