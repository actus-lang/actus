use std::fs;
use std::path::PathBuf;

use crate::ast::{PackDecl, Param, Program, Role, Stmt, TopLevelDecl, TypeName};
use crate::configuration::CompilerConfiguration;
use crate::lexer::{SourceSpan, Token, TokenKind, scan};
use crate::modules::{ModuleResolver, exports_module};
use crate::parser::parse;

use super::position::{LineIndex, LspPosition, LspRange};

#[derive(Clone, Debug)]
pub struct HoverInfo {
    pub contents: String,
    pub range: LspRange,
}

pub fn find_hover(uri: &str, source: &str, position: &LspPosition) -> Option<HoverInfo> {
    let tokens = scan(source).0;
    let offset = LineIndex::new(source).byte_offset(source, position)?;
    let (name, name_span) = identifier_at(&tokens, offset)?;
    let program = parse(tokens).ok()?;
    if is_primitive_name(&name) {
        return Some(HoverInfo {
            contents: format!("```actus\ntype {name}\n```"),
            range: range(source, name_span),
        });
    }
    let info = local_info(source, &program, &name, offset)
        .or_else(|| imported_info(uri, &program, &name))?;
    Some(HoverInfo {
        contents: format_markdown(&info, source, name_span.start),
        range: range(source, name_span),
    })
}

fn is_primitive_name(name: &str) -> bool {
    crate::ast::primitive_type(name).is_some()
}

#[derive(Clone, Debug)]
struct SymbolInfo {
    signature: String,
    span: SourceSpan,
    documentation: Option<String>,
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
    match declaration {
        TopLevelDecl::Verb(verb) if verb.name == name => Some(SymbolInfo {
            signature: format!("verb {}{}", verb.name, verb_signature(verb)),
            span: verb.span,
            documentation: verb.doc.clone(),
        }),
        TopLevelDecl::ExternalVerb(verb) if verb.name == name => Some(SymbolInfo {
            signature: format!("extern verb {}{}", verb.name, external_signature(verb)),
            span: verb.span,
            documentation: None,
        }),
        TopLevelDecl::Struct(definition) if definition.name == name => Some(SymbolInfo {
            signature: format!("struct {}", definition.name),
            span: definition.span,
            documentation: definition.doc.clone(),
        }),
        TopLevelDecl::Enum(definition) if definition.name == name => Some(SymbolInfo {
            signature: format!("enum {}", definition.name),
            span: definition.span,
            documentation: None,
        }),
        TopLevelDecl::Role(role) if role.name == name => Some(SymbolInfo {
            signature: format!("role {}", role.name),
            span: role.span,
            documentation: role.doc.clone(),
        }),
        TopLevelDecl::Pack(pack) if pack.name == name => Some(SymbolInfo {
            signature: pack_signature(pack),
            span: pack.span,
            documentation: None,
        }),
        _ => None,
    }
    .map(|mut info| {
        info.span = identifier_span(source, info.span, name).unwrap_or(info.span);
        info
    })
}

fn pack_field_info(source: &str, pack: &PackDecl, name: &str) -> Option<SymbolInfo> {
    let field = pack.fields.iter().find(|field| field.name == name)?;
    let width =
        crate::ast::primitive_type(&field.ty.name).and_then(|primitive| match primitive {
            crate::ast::PrimitiveType::Integer { width, .. } => Some(width),
            _ => None,
        })?;
    let mask = format_mask(width);
    Some(SymbolInfo {
        signature: format!(
            "{} {}: {} (offset: {}, width: {} bits, mask: {})",
            role_name(&field.role),
            field.name,
            type_name(&field.ty),
            field.offset,
            width,
            mask,
        ),
        span: identifier_span(source, field.span, name).unwrap_or(field.span),
        documentation: None,
    })
}

fn format_mask(width: u8) -> String {
    let mask = if width == 128 { u128::MAX } else { (1u128 << width) - 1 };
    let digits = usize::from(width.div_ceil(4)).max(2);
    format!("0x{mask:0digits$X}")
}

fn pack_signature(pack: &PackDecl) -> String {
    let endianness = match pack.endianness {
        crate::ast::LayoutEndianness::Little => "little",
        crate::ast::LayoutEndianness::Big => "big",
    };
    format!(
        "pack {} {{ storage: {}; layout {}; }}",
        pack.name,
        type_name(&pack.storage),
        endianness
    )
}

fn parameter_info(source: &str, params: &[Param], name: &str) -> Option<SymbolInfo> {
    params.iter().find(|param| param.name == name).map(|param| SymbolInfo {
        signature: format!("{} {}: {}", role_name(&param.role), name, type_name(&param.ty)),
        span: identifier_span(source, param.span, name).unwrap_or(param.span),
        documentation: None,
    })
}

fn block_info(source: &str, statements: &[Stmt], name: &str) -> Option<SymbolInfo> {
    for statement in statements {
        match statement {
            Stmt::OwnerDecl { role, name: declared, ty, span, .. } if declared == name => {
                let ty = ty.as_deref().unwrap_or("inferred");
                return Some(SymbolInfo {
                    signature: format!("{} {}: {}", role_name(role), name, ty),
                    span: identifier_span(source, *span, name).unwrap_or(*span),
                    documentation: None,
                });
            }
            Stmt::Loop(block) | Stmt::Block(block) => {
                if let Some(info) = block_info(source, &block.statements, name) {
                    return Some(info);
                }
            }
            _ => {}
        }
    }
    None
}

fn imported_info(uri: &str, program: &Program, name: &str) -> Option<SymbolInfo> {
    let current_path = file_uri_to_path(uri)?;
    let configuration = CompilerConfiguration::from_input_path_read_only(&current_path).ok()?;
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        let resolved = resolver.resolve(&import.path).ok()?;
        let exports = exports_module(&resolver, &import.path).ok()?;
        if let Some(path) = resolved.source_files().next() {
            let _symbol = exports
                .symbols
                .iter()
                .find(|symbol| symbol.name == name && symbol.source == path)?;
            let module_source = fs::read_to_string(path).ok()?;
            let module_program = parse(scan(&module_source).0).ok()?;
            let declaration = module_program
                .declarations
                .iter()
                .find(|declaration| declaration_name(declaration) == Some(name))?;
            return declaration_info(&module_source, declaration, name);
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

fn format_markdown(info: &SymbolInfo, source: &str, offset: usize) -> String {
    let documentation = info
        .documentation
        .as_deref()
        .filter(|documentation| !documentation.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| documentation_before(source, offset));
    if documentation.is_empty() {
        format!("```actus\n{}\n```", info.signature)
    } else {
        format!("```actus\n{}\n```\n\n{}", info.signature, documentation)
    }
}

fn documentation_before(source: &str, offset: usize) -> String {
    let prefix = &source[..offset.min(source.len())];
    let declaration_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let mut lines = Vec::new();
    for line in prefix[..declaration_start].lines().rev() {
        let trimmed = line.trim();
        if let Some(text) = trimmed.strip_prefix("///") {
            lines.push(text.trim().to_owned());
        } else if !trimmed.is_empty() {
            break;
        }
    }
    lines.reverse();
    lines.join("\n")
}

fn verb_signature(verb: &crate::ast::VerbDecl) -> String {
    format!("{}{}", parameters(&verb.params), return_type(verb.return_type.as_ref()))
}

fn external_signature(verb: &crate::ast::ExternalVerbDecl) -> String {
    format!("{}{}", parameters(&verb.params), return_type(verb.return_type.as_ref()))
}

fn parameters(params: &[Param]) -> String {
    let values = params
        .iter()
        .map(|param| format!("{} {}: {}", role_name(&param.role), param.name, type_name(&param.ty)))
        .collect::<Vec<_>>();
    format!("({})", values.join(", "))
}

fn return_type(return_type: Option<&crate::ast::ReturnType>) -> String {
    return_type.map_or_else(String::new, |return_type| {
        let access =
            if matches!(return_type.access, crate::ast::ReturnAccess::Abs) { "abs " } else { "" };
        format!(" -> {}{}", access, type_name(&return_type.ty))
    })
}

fn type_name(ty: &TypeName) -> String {
    if ty.arguments.is_empty() {
        ty.name.clone()
    } else {
        let arguments = ty.arguments.iter().map(type_name).collect::<Vec<_>>().join(", ");
        format!("{}[{}]", ty.name, arguments)
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}

fn identifier_at(tokens: &[Token], offset: usize) -> Option<(String, SourceSpan)> {
    tokens.iter().find_map(|token| {
        (token.span.start <= offset
            && offset <= token.span.end
            && matches!(
                token.kind,
                TokenKind::Identifier(_)
                    | TokenKind::IntType { .. }
                    | TokenKind::FloatType { .. }
                    | TokenKind::VoidType
            ))
        .then(|| match &token.kind {
            TokenKind::Identifier(name) => (name.clone(), token.span),
            TokenKind::IntType { signed, width } => {
                (format!("{}{}", if *signed { 'i' } else { 'u' }, width), token.span)
            }
            TokenKind::FloatType { width } => (format!("f{width}"), token.span),
            TokenKind::VoidType => ("Void".to_owned(), token.span),
            _ => unreachable!(),
        })
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

fn range(source: &str, span: SourceSpan) -> LspRange {
    let index = LineIndex::new(source);
    LspRange { start: index.position(source, span.start), end: index.position(source, span.end) }
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    uri.strip_prefix("file://").map(PathBuf::from)
}
