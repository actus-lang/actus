use crate::ast::{GenericParamKind, PackDecl, Param, Role, Stmt, TypeName, VerbContract};

use super::model::SymbolInfo;
use super::tokens::identifier_span;

pub(super) fn pack_field_info(source: &str, pack: &PackDecl, name: &str) -> Option<SymbolInfo> {
    let field = pack.fields.iter().find(|field| field.name == name)?;
    let (width, element_type, count) = pack_field_facts(field)?;
    let offset = field.offset_name.clone().unwrap_or_else(|| field.offset.to_string());
    let details = match (element_type, count) {
        (Some(element), Some(count)) => format!(", element: {element}, count: {count}"),
        _ => String::new(),
    };
    let mask =
        if width <= 128 { format!(", mask: {}", format_mask(width as u8)) } else { String::new() };
    Some(SymbolInfo {
        signature: format!(
            "{} {}: {} (offset: {}, width: {} bits{}{})",
            role_name(&field.role),
            field.name,
            type_name(&field.ty),
            offset,
            width,
            details,
            mask,
        ),
        span: identifier_span(source, field.span, name).unwrap_or(field.span),
        documentation: Some(pack_storage_details(pack)),
    })
}

fn pack_field_facts(field: &crate::ast::PackField) -> Option<(u16, Option<String>, Option<u32>)> {
    if field.ty.name == "Array" && field.ty.arguments.len() == 2 {
        let element = &field.ty.arguments[0];
        let count = field.ty.arguments[1].name.parse::<u32>().ok()?;
        let width =
            crate::ast::primitive_type(&element.name).and_then(|primitive| match primitive {
                crate::ast::PrimitiveType::Integer { width, .. } => {
                    u16::from(width).checked_mul(u16::try_from(count).ok()?)
                }
                _ => None,
            })?;
        return Some((width, Some(type_name(element)), Some(count)));
    }
    let width =
        crate::ast::primitive_type(&field.ty.name).and_then(|primitive| match primitive {
            crate::ast::PrimitiveType::Integer { width, .. } => Some(u16::from(width)),
            _ => None,
        })?;
    Some((width, None, None))
}

fn format_mask(width: u8) -> String {
    let mask = if width == 128 { u128::MAX } else { (1u128 << width) - 1 };
    let digits = usize::from(width.div_ceil(4)).max(2);
    format!("0x{mask:0digits$X}")
}

pub(super) fn pack_signature(pack: &PackDecl) -> String {
    let endianness = match pack.endianness {
        crate::ast::LayoutEndianness::Little => "little",
        crate::ast::LayoutEndianness::Big => "big",
    };
    format!(
        "pack {} {{ storage: {}; layout {}; }} [{}]",
        pack.name,
        type_name(pack.storage.type_name()),
        endianness,
        pack_storage_details(pack),
    )
}

fn pack_storage_details(pack: &PackDecl) -> String {
    match &pack.storage {
        crate::ast::PackStorage::ByteArray { element, capacity, .. } => {
            let width =
                crate::ast::primitive_type(&element.name).and_then(|primitive| match primitive {
                    crate::ast::PrimitiveType::Integer { width, .. } => Some(u64::from(width)),
                    _ => None,
                });
            match width.and_then(|value| value.checked_mul(*capacity)) {
                Some(bits) => format!("storage: {capacity} bytes, {bits} bits"),
                None => format!("storage: {capacity} bytes"),
            }
        }
        crate::ast::PackStorage::Scalar(type_name) => {
            let bits =
                crate::ast::primitive_type(&type_name.name).and_then(|primitive| match primitive {
                    crate::ast::PrimitiveType::Integer { width, .. } => Some(width),
                    _ => None,
                });
            bits.map_or_else(
                || "storage width unavailable".to_owned(),
                |value| format!("storage: {} bits", value),
            )
        }
    }
}

pub(super) fn parameter_info(source: &str, params: &[Param], name: &str) -> Option<SymbolInfo> {
    params.iter().find(|param| param.name == name).map(|param| SymbolInfo {
        signature: format!("{} {}: {}", role_name(&param.role), name, type_name(&param.ty)),
        span: identifier_span(source, param.span, name).unwrap_or(param.span),
        documentation: None,
    })
}

pub(super) fn block_info(source: &str, statements: &[Stmt], name: &str) -> Option<SymbolInfo> {
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
            Stmt::ForRange { binding, body, .. } => {
                if binding.name == name {
                    let ty = binding.ty.as_deref().unwrap_or("inferred");
                    return Some(SymbolInfo {
                        signature: format!("{} {}: {}", role_name(&binding.role), name, ty),
                        span: identifier_span(source, binding.span, name).unwrap_or(binding.span),
                        documentation: None,
                    });
                }
                if let Some(info) = block_info(source, &body.statements, name) {
                    return Some(info);
                }
            }
            Stmt::ForArray { binding, body, .. } => {
                if binding.name == name {
                    let ty = binding.ty.as_deref().unwrap_or("inferred");
                    return Some(SymbolInfo {
                        signature: format!("{} {}: {}", role_name(&binding.role), name, ty),
                        span: identifier_span(source, binding.span, name).unwrap_or(binding.span),
                        documentation: None,
                    });
                }
                if let Some(info) = block_info(source, &body.statements, name) {
                    return Some(info);
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                if let Some(info) = block_info(source, &then_branch.statements, name) {
                    return Some(info);
                }
                if let Some(crate::ast::IfBranch::Block(block)) = else_branch
                    && let Some(info) = block_info(source, &block.statements, name)
                {
                    return Some(info);
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn format_markdown(info: &SymbolInfo, source: &str, offset: usize) -> String {
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

pub(super) fn declaration_signature(declaration: &crate::ast::TopLevelDecl) -> Option<String> {
    match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => Some(format!(
            "verb {}{}{}",
            verb.name,
            generic_label(&verb.generic_parameters),
            verb_signature(verb)
        )),
        crate::ast::TopLevelDecl::ExternalVerb(verb) => Some(format!(
            "extern verb {}{}{}",
            verb.name,
            generic_label(&verb.generic_parameters),
            external_signature(verb)
        )),
        crate::ast::TopLevelDecl::Struct(definition) => Some(format!(
            "struct {}{}",
            definition.name,
            generic_label(&definition.generic_parameters)
        )),
        crate::ast::TopLevelDecl::Enum(definition) => Some(format!(
            "enum {}{}",
            definition.name,
            generic_label(&definition.generic_parameters)
        )),
        crate::ast::TopLevelDecl::Role(role) => Some(format!("role {}", role.name)),
        crate::ast::TopLevelDecl::Pack(pack) => Some(pack_signature(pack)),
        _ => None,
    }
}

fn generic_label(parameters: &[crate::ast::GenericParam]) -> String {
    if parameters.is_empty() {
        return String::new();
    }
    let values = parameters
        .iter()
        .map(|parameter| match &parameter.kind {
            GenericParamKind::Type => parameter.name.clone(),
            GenericParamKind::Const { domain } => {
                format!("{}: {}", parameter.name, type_name(domain))
            }
        })
        .collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
}

pub(super) fn declaration_documentation(declaration: &crate::ast::TopLevelDecl) -> Option<String> {
    match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => {
            contract_documentation(verb.doc.as_deref(), verb.contract.as_ref())
        }
        crate::ast::TopLevelDecl::ExternalVerb(verb) => {
            contract_documentation(verb.doc.as_deref(), verb.contract.as_ref())
        }
        crate::ast::TopLevelDecl::Struct(definition) => definition.doc.clone(),
        crate::ast::TopLevelDecl::Role(role) => role.doc.clone(),
        _ => None,
    }
}

pub(crate) fn contract_documentation(
    documentation: Option<&str>,
    contract: Option<&VerbContract>,
) -> Option<String> {
    let Some(contract) = contract else { return documentation.map(str::to_owned) };
    let mut rendered = String::from("**Contract**\n");
    for section in &contract.sections {
        rendered.push_str("\n### ");
        rendered.push_str(contract_section_name(section.kind));
        rendered.push_str("\n\n");
        rendered.push_str(&section.text);
        rendered.push('\n');
    }
    Some(rendered)
}

fn contract_section_name(kind: crate::ast::VerbContractSectionKind) -> &'static str {
    match kind {
        crate::ast::VerbContractSectionKind::Purpose => "purpose",
        crate::ast::VerbContractSectionKind::Inputs => "inputs",
        crate::ast::VerbContractSectionKind::Outputs => "outputs",
        crate::ast::VerbContractSectionKind::Ownership => "ownership",
        crate::ast::VerbContractSectionKind::Invariants => "invariants",
        crate::ast::VerbContractSectionKind::Errors => "errors",
        crate::ast::VerbContractSectionKind::SideEffects => "side_effects",
        crate::ast::VerbContractSectionKind::Abi => "abi",
    }
}

pub(super) fn type_name(ty: &TypeName) -> String {
    if ty.arguments.is_empty() {
        ty.name.clone()
    } else {
        let arguments = ty.arguments.iter().map(type_name).collect::<Vec<_>>().join(", ");
        format!("{}[{}]", ty.name, arguments)
    }
}

pub(super) fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}
