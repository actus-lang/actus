use std::collections::HashMap;
use std::path::PathBuf;

use crate::ast::{Program, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::lexer::{SourceSpan, Token};
use crate::modules::ModuleResolver;
use crate::parser::parse;
use crate::semantic::{AccessState, OwnershipState};
use crate::target::TargetSpec;

use super::super::position::{LineIndex, LspPosition};
use super::formatting::{
    block_info, declaration_documentation, declaration_signature, format_markdown, pack_field_info,
    parameter_info, role_name,
};
use super::model::SymbolInfo;
use super::tokens::{file_uri_to_path, identifier_at, operator_at, range};

pub(crate) fn find_hover(
    uri: &str,
    source: &str,
    position: &LspPosition,
    overlays: &HashMap<PathBuf, String>,
    target: &TargetSpec,
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
    let program = crate::semantic::filter_program_for_target(&parse(tokens.clone()).ok()?, target);
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
    if let Some(info) = semantic_binding_info(source, &program, &name, offset) {
        return Some(super::HoverInfo {
            contents: format_markdown(&info, source, info.span.start),
            range: range(source, name_span),
        });
    }
    let mut info = intrinsic_info(&name)
        .or_else(|| local_info(source, &program, &name, offset))
        .or_else(|| imported_info(uri, &program, &name, overlays, target))?;
    append_target_details(&mut info, target);
    Some(super::HoverInfo {
        contents: format_markdown(&info, source, name_span.start),
        range: range(source, name_span),
    })
}

fn append_target_details(info: &mut SymbolInfo, target: &TargetSpec) {
    if info.signature.starts_with("pack ") {
        let details = format!(
            "Target data model: `{}`; pointer width `{:?}`; endianness `{:?}`; entry contract `{}`.",
            target.triple(),
            target.pointer_width,
            target.endianness,
            match target.entry_contract() {
                crate::target::EntryContract::Hosted => "hosted",
                crate::target::EntryContract::Freestanding => "freestanding",
            },
        );
        info.documentation = Some(match info.documentation.take() {
            Some(documentation) => format!("{documentation}\n\n{details}"),
            None => details,
        });
    }
}

fn semantic_binding_info(
    source: &str,
    program: &Program,
    name: &str,
    offset: usize,
) -> Option<SymbolInfo> {
    let model = crate::semantic::analyze(program).ok()?;
    let (index, binding) = model
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, binding)| binding.name == name && binding.span.start <= offset)
        .max_by_key(|(_, binding)| binding.span.start)?;
    let type_name = model
        .binding_type_names
        .get(&index)
        .map(format_type_name)
        .or_else(|| binding.ty.map(|ty| ty.spec().name.to_owned()))?;
    let identifier =
        super::tokens::identifier_span(source, binding.span, name).unwrap_or(binding.span);
    let documentation = indexed_type_documentation(&type_name);
    Some(SymbolInfo {
        signature: format!(
            "{} {name}: {type_name} [ownership: {}; access: {}]",
            role_name(&binding.role),
            ownership_name(&binding.ownership),
            access_name(&binding.access),
        ),
        span: identifier,
        documentation,
    })
}

fn indexed_type_documentation(type_name: &str) -> Option<String> {
    if type_name == "Buffer" {
        return Some(
            "Compiler facts: element type `u8`; valid dynamic indexes satisfy `0 <= index < length`; out-of-bounds access uses the deterministic runtime trap.".to_owned(),
        );
    }
    let arguments =
        type_name.strip_prefix("Array[")?.strip_suffix(']')?.split(", ").collect::<Vec<_>>();
    (arguments.len() == 2).then(|| format!(
        "Compiler facts: element type `{}`; capacity `{}`; valid dynamic indexes satisfy `0 <= index < {}`; out-of-bounds access uses the deterministic runtime trap.",
        arguments[0], arguments[1], arguments[1]
    ))
}

fn format_type_name(type_name: &crate::ast::TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(format_type_name).collect::<Vec<_>>().join(", ")
    )
}

fn ownership_name(state: &OwnershipState) -> String {
    match state {
        OwnershipState::Active => "Active".to_owned(),
        OwnershipState::PartiallyMoved { fields } => {
            format!("PartiallyMoved({})", fields.join(", "))
        }
        OwnershipState::Moved => "Moved".to_owned(),
        OwnershipState::Dropped => "Dropped".to_owned(),
    }
}

fn access_name(state: &AccessState) -> String {
    match state {
        AccessState::Mutable => "Mutable".to_owned(),
        AccessState::Frozen { borrow_ids } => format!("Frozen({})", borrow_ids.len()),
        AccessState::Suspended { loan_id } => format!("Suspended({loan_id})"),
    }
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
    target: &TargetSpec,
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
            let module_program = crate::semantic::filter_program_for_target(
                &parse(crate::lexer::scan(&module_source).0).ok()?,
                target,
            );
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
