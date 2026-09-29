use std::collections::HashMap;
use std::path::PathBuf;

use crate::ast::{Program, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::lexer::{SourceSpan, Token};
use crate::modules::ModuleResolver;
use crate::parser::parse;

use super::super::position::{LineIndex, LspPosition};
use super::formatting::{
    block_info, declaration_documentation, declaration_signature, format_markdown, pack_field_info,
    parameter_info,
};
use super::model::SymbolInfo;
use super::tokens::{file_uri_to_path, identifier_at, operator_at, range};

pub(crate) fn find_hover(
    uri: &str,
    source: &str,
    position: &LspPosition,
    overlays: &HashMap<PathBuf, String>,
) -> Option<super::HoverInfo> {
    let tokens = crate::lexer::scan(source).0;
    let offset = LineIndex::new(source).byte_offset(source, position)?;
    if let Some((operator, span)) = operator_at(&tokens, offset) {
        return Some(super::HoverInfo {
            contents: operator_documentation(operator).to_owned(),
            range: range(source, span),
        });
    }
    let (name, name_span) = identifier_at(&tokens, offset)?;
    let program = parse(tokens.clone()).ok()?;
    if crate::ast::primitive_type(&name).is_some() {
        return Some(super::HoverInfo {
            contents: format!("```actus\ntype {name}\n```"),
            range: range(source, name_span),
        });
    }
    if name == "place"
        && let Some(info) = arena_place_info(source, &program, &tokens, name_span, offset)
    {
        return Some(super::HoverInfo {
            contents: format_markdown(&info, source, info.span.start),
            range: range(source, name_span),
        });
    }
    let info = intrinsic_info(&name)
        .or_else(|| local_info(source, &program, &name, offset))
        .or_else(|| imported_info(uri, &program, &name, overlays))?;
    Some(super::HoverInfo {
        contents: format_markdown(&info, source, name_span.start),
        range: range(source, name_span),
    })
}

fn operator_documentation(operator: &str) -> &'static str {
    match operator {
        "&&" | "||" => {
            "```actus\nlogical operator: Bool -> Bool\n```\n\nShort-circuits the right operand."
        }
        "as" => {
            "```actus\nchecked integer cast: expr as Type\n```\n\nRejects constant overflow and traps on dynamic overflow."
        }
        "==" | "!=" => {
            "```actus\nequality operator -> Bool\n```\n\nRequires compatible operand families without implicit conversion."
        }
        "%" => {
            "```actus\ninteger remainder\n```\n\nZero divisors are rejected when statically provable or trapped at runtime."
        }
        "<<" | ">>" => {
            "```actus\ninteger shift\n```\n\nThe count must be unsigned and smaller than the destination width."
        }
        "<" | "<=" | ">" | ">=" => {
            "```actus\nrelational operator -> Bool\n```\n\nSignedness and numeric family must match."
        }
        _ => {
            "```actus\ninteger bitwise operator\n```\n\nPreserves the validated integer family and width."
        }
    }
}

fn intrinsic_info(name: &str) -> Option<SymbolInfo> {
    let (signature, documentation) = match name {
        "print" => (
            "print(abs text: Buffer) -> Result[Int, IoError]",
            "Writes exactly the Buffer live length, including embedded zero bytes; no null terminator is required.",
        ),
        "println" => (
            "println(abs text: Buffer) -> Result[Int, IoError]",
            "Writes the Buffer live length and a line ending; output is length-aware and binary-safe.",
        ),
        _ => return None,
    };
    Some(SymbolInfo {
        signature: signature.to_owned(),
        span: SourceSpan::new(0, 0),
        documentation: Some(documentation.to_owned()),
    })
}

fn arena_place_info(
    source: &str,
    program: &Program,
    tokens: &[Token],
    method_span: SourceSpan,
    offset: usize,
) -> Option<SymbolInfo> {
    let method_index = tokens.iter().position(|token| token.span == method_span)?;
    let receiver = tokens.get(method_index.checked_sub(2)?)?;
    if !matches!(tokens.get(method_index.checked_sub(1)?)?.kind, crate::lexer::TokenKind::Dot) {
        return None;
    }
    let crate::lexer::TokenKind::Identifier(receiver_name) = &receiver.kind else { return None };
    let owner = local_info(source, program, receiver_name, offset)?;
    let type_name = owner.signature.split_once(": ")?.1;
    let capacity = type_name.strip_prefix("Arena[")?.strip_suffix(']')?;
    Some(SymbolInfo {
        signature: format!("Arena[{capacity}].place(value) -> ins T"),
        span: method_span,
        documentation: Some(
            "Places a value in the arena using aligned bump-pointer storage.".to_owned(),
        ),
    })
}

fn local_info(source: &str, program: &Program, name: &str, offset: usize) -> Option<SymbolInfo> {
    for declaration in &program.declarations {
        if let Some(info) = declaration_info(source, declaration, name) {
            return Some(info);
        }
        if let TopLevelDecl::Pack(pack) = declaration
            && let Some(info) = pack_field_info(source, pack, name)
        {
            return Some(info);
        }
        if let TopLevelDecl::Verb(verb) = declaration
            && verb.span.start <= offset
            && offset <= verb.span.end
        {
            if let Some(info) = parameter_info(source, &verb.params, name) {
                return Some(info);
            }
            if let Some(info) = block_info(source, &verb.body.statements, name) {
                return Some(info);
            }
        }
    }
    None
}

fn declaration_info(source: &str, declaration: &TopLevelDecl, name: &str) -> Option<SymbolInfo> {
    let matches_name = match declaration {
        TopLevelDecl::Verb(value) => value.name == name,
        TopLevelDecl::ExternalVerb(value) => value.name == name,
        TopLevelDecl::Struct(value) => value.name == name,
        TopLevelDecl::Enum(value) => value.name == name,
        TopLevelDecl::Role(value) => value.name == name,
        TopLevelDecl::Pack(value) => value.name == name,
        _ => false,
    };
    if !matches_name {
        return None;
    }
    let span = match declaration {
        TopLevelDecl::Verb(value) => value.span,
        TopLevelDecl::ExternalVerb(value) => value.span,
        TopLevelDecl::Struct(value) => value.span,
        TopLevelDecl::Enum(value) => value.span,
        TopLevelDecl::Role(value) => value.span,
        TopLevelDecl::Pack(value) => value.span,
        _ => return None,
    };
    Some(SymbolInfo {
        signature: declaration_signature(declaration)?,
        span: super::tokens::identifier_span(source, span, name).unwrap_or(span),
        documentation: declaration_documentation(declaration),
    })
}

fn imported_info(
    uri: &str,
    program: &Program,
    name: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Option<SymbolInfo> {
    let current_path = file_uri_to_path(uri)?;
    let configuration = CompilerConfiguration::from_input_path_read_only(&current_path).ok()?;
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        let resolved = resolver.resolve(&import.path).ok()?;
        let exports =
            crate::modules::exports_module_with_overlays(&resolver, &import.path, overlays).ok()?;
        for path in resolved.source_files() {
            if !exports.symbols.iter().any(|symbol| symbol.name == name && symbol.source == path) {
                continue;
            }
            let module_source = super::super::module_scope::source_for_path(path, overlays)?;
            let module_program = parse(crate::lexer::scan(&module_source).0).ok()?;
            if let Some(declaration) = module_program
                .declarations
                .iter()
                .find(|declaration| declaration_name(declaration) == Some(name))
            {
                return declaration_info(&module_source, declaration, name);
            }
        }
    }
    None
}

fn declaration_name(declaration: &TopLevelDecl) -> Option<&str> {
    match declaration {
        TopLevelDecl::Verb(value) => Some(&value.name),
        TopLevelDecl::ExternalVerb(value) => Some(&value.name),
        TopLevelDecl::Struct(value) => Some(&value.name),
        TopLevelDecl::Enum(value) => Some(&value.name),
        TopLevelDecl::Role(value) => Some(&value.name),
        TopLevelDecl::Pack(value) => Some(&value.name),
        _ => None,
    }
}
