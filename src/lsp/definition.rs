use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{Block, Param, Program, Stmt, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::lexer::{SourceSpan, Token, TokenKind, scan};
use crate::modules::ModuleResolver;
use crate::parser::parse;

use super::position::{LineIndex, LspPosition, LspRange};
use super::uri::{file_uri_to_path, path_to_file_uri};

#[derive(Clone, Debug)]
pub struct DefinitionLocation {
    pub uri: String,
    pub range: LspRange,
}

pub fn find_definition(
    uri: &str,
    source: &str,
    position: &LspPosition,
    overlays: &HashMap<PathBuf, String>,
) -> Option<DefinitionLocation> {
    let tokens = scan(source).0;
    let offset = LineIndex::new(source).byte_offset(source, position)?;
    let name = identifier_at(&tokens, offset)?;
    let program = parse(tokens).ok()?;
    let current_path = file_uri_to_path(uri)?;
    if let Some(span) = local_definition(source, &program, &name, offset) {
        return Some(location(uri, source, span));
    }
    imported_definition(&current_path, &program, &name, overlays)
}

fn local_definition(
    source: &str,
    program: &Program,
    name: &str,
    use_offset: usize,
) -> Option<SourceSpan> {
    let mut definitions = Vec::new();
    for declaration in &program.declarations {
        collect_top_level_definition(source, declaration, name, &mut definitions);
        if let TopLevelDecl::Pack(pack) = declaration {
            collect_pack_field_definitions(source, pack, name, &mut definitions);
        }
        if let TopLevelDecl::Verb(verb) = declaration
            && verb.span.start <= use_offset
            && use_offset <= verb.span.end
        {
            collect_params(source, &verb.params, name, &mut definitions);
            collect_block_definitions(source, &verb.body, name, &mut definitions);
        }
    }
    definitions.into_iter().filter(|span| span.start <= use_offset).max_by_key(|span| span.start)
}

fn collect_pack_field_definitions(
    source: &str,
    pack: &crate::ast::PackDecl,
    name: &str,
    definitions: &mut Vec<SourceSpan>,
) {
    for field in &pack.fields {
        if field.name == name
            && let Some(identifier) = identifier_span(source, field.span, name)
        {
            definitions.push(identifier);
        }
    }
}

fn collect_top_level_definition(
    source: &str,
    declaration: &TopLevelDecl,
    name: &str,
    definitions: &mut Vec<SourceSpan>,
) {
    let (declared_name, span) = match declaration {
        TopLevelDecl::Verb(value) => (&value.name, value.span),
        TopLevelDecl::ExternalVerb(value) => (&value.name, value.span),
        TopLevelDecl::Struct(value) => (&value.name, value.span),
        TopLevelDecl::Enum(value) => (&value.name, value.span),
        TopLevelDecl::Role(value) => (&value.name, value.span),
        TopLevelDecl::Pack(value) => (&value.name, value.span),
        _ => return,
    };
    if declared_name == name
        && let Some(identifier) = identifier_span(source, span, name)
    {
        definitions.push(identifier);
    }
}

fn collect_params(source: &str, params: &[Param], name: &str, definitions: &mut Vec<SourceSpan>) {
    for parameter in params {
        if parameter.name == name
            && let Some(identifier) = identifier_span(source, parameter.span, name)
        {
            definitions.push(identifier);
        }
    }
}

fn collect_block_definitions(
    source: &str,
    block: &Block,
    name: &str,
    definitions: &mut Vec<SourceSpan>,
) {
    for statement in &block.statements {
        match statement {
            Stmt::OwnerDecl { name: declared, span, .. } if declared == name => {
                if let Some(identifier) = identifier_span(source, *span, name) {
                    definitions.push(identifier);
                }
            }
            Stmt::Loop(nested) | Stmt::Block(nested) => {
                collect_block_definitions(source, nested, name, definitions)
            }
            Stmt::If { then_branch, else_branch, .. } => {
                collect_block_definitions(source, then_branch, name, definitions);
                if let Some(crate::ast::IfBranch::Block(block)) = else_branch {
                    collect_block_definitions(source, block, name, definitions);
                }
            }
            _ => {}
        }
    }
}

fn imported_definition(
    current_path: &Path,
    program: &Program,
    name: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Option<DefinitionLocation> {
    let configuration = CompilerConfiguration::from_input_path_read_only(current_path).ok()?;
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    );
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        let resolved = resolver.resolve(&import.path).ok()?;
        let exports =
            crate::modules::exports_module_with_overlays(&resolver, &import.path, overlays).ok()?;
        if !exports.symbols.iter().any(|symbol| symbol.name == name) {
            continue;
        }
        for path in resolved.source_files() {
            let module_source = super::module_scope::source_for_path(path, overlays)?;
            let module_program = parse(scan(&module_source).0).ok()?;
            if !exports.symbols.iter().any(|symbol| symbol.name == name && symbol.source == path) {
                continue;
            }
            if let Some(span) = top_level_span(&module_source, &module_program, name) {
                return Some(location(&path_to_file_uri(path), &module_source, span));
            }
        }
    }
    None
}

fn top_level_span(source: &str, program: &Program, name: &str) -> Option<SourceSpan> {
    program.declarations.iter().find_map(|declaration| {
        let (declared_name, span) = match declaration {
            TopLevelDecl::Verb(value) => (&value.name, value.span),
            TopLevelDecl::ExternalVerb(value) => (&value.name, value.span),
            TopLevelDecl::Struct(value) => (&value.name, value.span),
            TopLevelDecl::Enum(value) => (&value.name, value.span),
            TopLevelDecl::Role(value) => (&value.name, value.span),
            TopLevelDecl::Pack(value) => (&value.name, value.span),
            _ => return None,
        };
        (declared_name == name).then(|| identifier_span(source, span, name)).flatten()
    })
}

fn identifier_at(tokens: &[Token], offset: usize) -> Option<String> {
    tokens.iter().find_map(|token| {
        if token.span.start <= offset
            && offset <= token.span.end
            && let TokenKind::Identifier(name) = &token.kind
        {
            return Some(name.clone());
        }
        None
    })
}

fn identifier_span(source: &str, span: SourceSpan, name: &str) -> Option<SourceSpan> {
    let (tokens, _) = scan(source.get(span.start..span.end)?);
    tokens.into_iter().find_map(|token| match token.kind {
        TokenKind::Identifier(identifier) if identifier == name => {
            Some(SourceSpan::new(span.start + token.span.start, span.start + token.span.end))
        }
        _ => None,
    })
}

fn location(uri: &str, source: &str, span: SourceSpan) -> DefinitionLocation {
    let index = LineIndex::new(source);
    DefinitionLocation {
        uri: uri.to_owned(),
        range: LspRange {
            start: index.position(source, span.start),
            end: index.position(source, span.end),
        },
    }
}
