use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, Program, TopLevelDecl, TypeName, VerbDecl};

mod constructors;
mod expression;
mod layout;
mod support;

use expression::SharedExpressionProvider;
use layout::ProgramLayouts;
use support::{backend_value_type, unsupported};

use super::abi::BackendEnumLayout;
use super::{
    BackendBlockId, BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupProvider, BackendCleanupResolvedDescriptor, BackendCleanupResult,
    BackendContractError, BackendExternal, BackendInstruction, BackendLayout,
    BackendLayoutProvider, BackendModule, BackendParameter, BackendSignature,
    BackendSymbolBindings, BackendValueType, SymbolIdentity, lower_parameter, lower_return,
    lower_verb_signature, lower_verb_with_expressions, lower_verb_with_expressions_as,
};

struct BuiltinLayouts;

impl BackendLayoutProvider for BuiltinLayouts {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
        matches!(type_name.name.as_str(), "Arena" | "Buffer" | "Region" | "String")
            .then_some(host_pointer_layout())
    }

    fn enum_layout_for(&self, type_name: &TypeName) -> Option<BackendEnumLayout> {
        (type_name.name == "Result" && type_name.arguments.len() == 2)
            .then(|| result_layout(self, type_name.arguments.first()?))
            .flatten()
    }
}

impl BackendCleanupProvider for BuiltinLayouts {
    fn resolve_binding(
        &self,
        type_name: Option<&TypeName>,
        builtin: Option<crate::ast::BuiltinType>,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        let bridge = if builtin == Some(crate::ast::BuiltinType::Buffer) {
            Some(BackendCleanupBridge::BufferDrop)
        } else if type_name.is_some_and(|name| name.name == "Region") {
            Some(BackendCleanupBridge::RegionDrop)
        } else {
            None
        };
        Ok(bridge.map(|bridge| BackendCleanupResolvedDescriptor {
            descriptor: BackendCleanupBridgeDescriptor {
                bridge,
                argument: if bridge == BackendCleanupBridge::BufferDrop {
                    BackendCleanupArgument::Value
                } else {
                    BackendCleanupArgument::Address
                },
                result: if bridge == BackendCleanupBridge::RegionDrop {
                    BackendCleanupResult::Status
                } else {
                    BackendCleanupResult::Ignored
                },
            },
            layout: host_pointer_layout(),
            field: None,
        }))
    }

    fn resolve_field(
        &self,
        _root_type: &TypeName,
        _path: &str,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError> {
        Ok(None)
    }
}

fn host_pointer_layout() -> BackendLayout {
    let pointer_bytes = (usize::BITS / 8) as u32;
    BackendLayout { size: pointer_bytes, alignment: pointer_bytes }
}

fn result_layout(layouts: &BuiltinLayouts, payload: &TypeName) -> Option<BackendEnumLayout> {
    let payload = builtin_value_layout(layouts, payload)?;
    let payload_offset = align_up(4, payload.alignment);
    let alignment = 4u32.max(payload.alignment);
    let size = align_up(payload_offset.checked_add(payload.size)?, alignment);
    Some(BackendEnumLayout {
        layout: BackendLayout { size, alignment },
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset,
        payload: Some(payload),
        niche_pointer: false,
    })
}

fn builtin_value_layout(layouts: &BuiltinLayouts, type_name: &TypeName) -> Option<BackendLayout> {
    match type_name.name.as_str() {
        "Bool" | "u8" | "i8" => Some(BackendLayout { size: 1, alignment: 1 }),
        "u16" | "i16" => Some(BackendLayout { size: 2, alignment: 2 }),
        "Int" | "u32" | "i32" => Some(BackendLayout { size: 4, alignment: 4 }),
        "Usize" | "u64" | "i64" => Some(BackendLayout { size: 8, alignment: 8 }),
        "Buffer" | "String" | "Arena" | "Region" => Some(host_pointer_layout()),
        _ => layouts.layout_for(type_name),
    }
}

fn align_up(value: u32, alignment: u32) -> u32 {
    let remainder = value % alignment;
    if remainder == 0 { value } else { value + alignment - remainder }
}

#[doc(hidden)]
pub fn lower_scalar_program(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
) -> Result<BackendModule, BackendContractError> {
    lower_program_to_backend_module(namespace, program, entry_name, None)
}

pub(crate) fn lower_program_to_backend_module(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BackendModule, BackendContractError> {
    let layouts = ProgramLayouts::new(program);
    lower_program_to_backend_module_with_layouts(namespace, program, entry_name, bindings, &layouts)
}

pub(crate) fn lower_program_to_backend_module_with_layouts(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendModule, BackendContractError> {
    let (verbs, externals) = scalar_declarations(program)?;
    let signatures = verbs
        .iter()
        .map(|verb| lower_verb_signature(namespace, verb, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let external_signatures = externals
        .iter()
        .map(|external| lower_external_signature(external, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let return_types = shared_return_types(&verbs, &externals, &signatures, &external_signatures);
    let return_payload_types = shared_return_payload_types(&verbs, &externals, layouts);
    let semantic = crate::semantic::analyze(program).map_err(|error| {
        unsupported(
            crate::lexer::SourceSpan::new(0, 0),
            format!("semantic analysis failed: {error:?}"),
        )
    })?;
    let cleanup_provider = BuiltinLayouts;
    let cleanup_plans = super::lower_semantic_cleanup_with_provider(&semantic, &cleanup_provider)?;
    let external_symbols = scalar_external_symbols(namespace, &externals, bindings)?;
    let mut functions = Vec::with_capacity(verbs.len());
    for verb in verbs {
        functions.push(lower_shared_verb(
            namespace,
            verb,
            entry_name,
            layouts,
            &cleanup_plans,
            &return_types,
            &return_payload_types,
            &external_symbols,
            bindings,
        )?);
    }
    let mut module = BackendModule {
        externals: build_backend_externals(&externals, external_signatures, &external_symbols)?,
        functions,
    };
    append_buffer_allocate_external(&mut module.externals)?;
    module.validate()?;
    Ok(module)
}

fn build_backend_externals(
    externals: &[&ExternalVerbDecl],
    signatures: Vec<BackendSignature>,
    symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<Vec<BackendExternal>, BackendContractError> {
    externals
        .iter()
        .zip(signatures)
        .map(|(external, signature)| scalar_external(external, signature, symbols))
        .collect()
}

fn append_buffer_allocate_external(
    externals: &mut Vec<BackendExternal>,
) -> Result<(), BackendContractError> {
    if externals
        .iter()
        .any(|external| external.symbol.as_str() == crate::runtime::BUFFER_ALLOCATE_SYMBOL)
    {
        return Ok(());
    }
    externals.push(buffer_allocate_external()?);
    Ok(())
}

fn buffer_allocate_external() -> Result<BackendExternal, BackendContractError> {
    Ok(BackendExternal {
        symbol: SymbolIdentity::external_raw(crate::runtime::BUFFER_ALLOCATE_SYMBOL)
            .map_err(|error| unsupported(crate::lexer::SourceSpan::new(0, 0), error.to_string()))?,
        parameters: vec![BackendParameter {
            name: "length".to_owned(),
            role: crate::ast::Role::Erg,
            ty: BackendValueType::Integer { signed: false, width: 64 },
            passing: super::BackendPassingMode::Direct,
        }],
        return_value: super::BackendReturn {
            role: Some(crate::ast::Role::Erg),
            ty: BackendValueType::Pointer,
            passing: super::BackendReturnMode::Direct,
        },
        runtime_pointer_return: false,
    })
}

fn scalar_declarations(
    program: &Program,
) -> Result<(Vec<&VerbDecl>, Vec<&ExternalVerbDecl>), BackendContractError> {
    let mut verbs = Vec::new();
    let mut externals = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => verbs.push(verb),
            TopLevelDecl::ExternalVerb(external) => externals.push(external),
            _ => {}
        }
    }
    Ok((verbs, externals))
}

fn scalar_external_symbols(
    namespace: &str,
    externals: &[&ExternalVerbDecl],
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BTreeMap<String, SymbolIdentity>, BackendContractError> {
    externals
        .iter()
        .map(|external| {
            let source = external.native_symbol.as_deref().unwrap_or(&external.name);
            let symbol = bindings
                .map(|bindings| bindings.external_symbol(namespace, source))
                .unwrap_or(source);
            Ok((
                external.name.clone(),
                SymbolIdentity::external_raw(symbol)
                    .map_err(|error| unsupported(external.span, error.to_string()))?,
            ))
        })
        .collect()
}

fn scalar_external(
    external: &ExternalVerbDecl,
    signature: BackendSignature,
    symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<BackendExternal, BackendContractError> {
    let symbol = symbols.get(&external.name).cloned().ok_or_else(|| {
        unsupported(external.span, format!("external symbol `{}` is unavailable", external.name))
    })?;
    Ok(BackendExternal {
        symbol,
        parameters: signature.parameters,
        return_value: signature.return_value,
        runtime_pointer_return: !external.module_import
            && matches!(
                external.return_type.as_ref().map(|return_type| return_type.ty.name.as_str()),
                Some("Result" | "Array" | "Pack" | "Struct" | "Region")
            ),
    })
}

fn lower_external_signature(
    external: &ExternalVerbDecl,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendSignature, BackendContractError> {
    let symbol = SymbolIdentity::external_raw(&external.name)
        .map_err(|error| unsupported(external.span, error.to_string()))?;
    let parameters = external
        .params
        .iter()
        .map(|parameter| lower_parameter(parameter, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let return_value = lower_return(external.return_type.as_ref(), layouts)?;
    Ok(BackendSignature { symbol, parameters, return_value })
}

fn shared_return_types(
    verbs: &[&VerbDecl],
    externals: &[&ExternalVerbDecl],
    signatures: &[BackendSignature],
    external_signatures: &[BackendSignature],
) -> BTreeMap<String, Option<BackendValueType>> {
    verbs
        .iter()
        .zip(signatures)
        .map(|(verb, signature)| {
            (
                verb.name.clone(),
                (signature.return_value.ty != BackendValueType::Unit)
                    .then_some(signature.return_value.ty),
            )
        })
        .chain(externals.iter().zip(external_signatures).map(|(external, signature)| {
            (
                external.name.clone(),
                (signature.return_value.ty != BackendValueType::Unit)
                    .then_some(signature.return_value.ty),
            )
        }))
        .collect()
}

fn shared_return_payload_types(
    verbs: &[&VerbDecl],
    externals: &[&ExternalVerbDecl],
    layouts: &impl BackendLayoutProvider,
) -> BTreeMap<String, [Option<BackendValueType>; 2]> {
    verbs
        .iter()
        .filter_map(|verb| {
            result_payload_types(
                verb.return_type.as_ref().map(|return_type| &return_type.ty),
                layouts,
            )
            .map(|types| (verb.name.clone(), types))
        })
        .chain(externals.iter().filter_map(|external| {
            result_payload_types(
                external.return_type.as_ref().map(|return_type| &return_type.ty),
                layouts,
            )
            .map(|types| (external.name.clone(), types))
        }))
        .collect()
}

fn result_payload_types(
    type_name: Option<&TypeName>,
    layouts: &impl BackendLayoutProvider,
) -> Option<[Option<BackendValueType>; 2]> {
    let type_name = type_name?;
    (type_name.name == "Result" && type_name.arguments.len() == 2).then(|| {
        [
            backend_value_type(layouts, &type_name.arguments[0]),
            backend_value_type(layouts, &type_name.arguments[1]),
        ]
    })
}

fn lower_shared_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
    layouts: &impl BackendLayoutProvider,
    cleanup_plans: &[super::BackendCleanupPlan],
    return_types: &BTreeMap<String, Option<BackendValueType>>,
    return_payload_types: &BTreeMap<String, [Option<BackendValueType>; 2]>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<super::BackendFunction, BackendContractError> {
    let mut provider = SharedExpressionProvider::new(
        namespace,
        verb,
        layouts,
        return_types,
        return_payload_types,
        external_symbols,
        bindings,
        cleanup_plans,
    );
    let mut function = match entry_name.filter(|name| *name == verb.name) {
        Some(_) => lower_verb_with_expressions_as(
            SymbolIdentity::entry(&verb.name)
                .map_err(|error| unsupported(verb.span, error.to_string()))?,
            verb,
            layouts,
            &mut provider,
            BackendBlockId(0),
        ),
        None => {
            lower_verb_with_expressions(namespace, verb, layouts, &mut provider, BackendBlockId(0))
        }
    }?;
    function.cleanup = cleanup_plans
        .iter()
        .filter(|plan| plan.span.start >= verb.span.start && plan.span.end <= verb.span.end)
        .cloned()
        .collect();
    append_parameter_cleanup(&mut function, &provider);
    function.validate()?;
    Ok(function)
}

type ParameterCleanup =
    (String, super::BackendCleanupAction, super::BackendCleanupOperand, crate::lexer::SourceSpan);

fn append_parameter_cleanup(
    function: &mut super::BackendFunction,
    provider: &SharedExpressionProvider<'_>,
) {
    let cleanup = parameter_cleanup(function, provider);
    if cleanup.is_empty() {
        return;
    }
    for block in &mut function.blocks {
        let instructions = std::mem::take(&mut block.instructions);
        let mut lowered = Vec::with_capacity(instructions.len() + cleanup.len());
        for instruction in instructions {
            let returned_value = match &instruction {
                BackendInstruction::Return { value, .. } => *value,
                BackendInstruction::Fail { .. } => None,
                _ => None,
            };
            if matches!(
                instruction,
                BackendInstruction::Return { .. } | BackendInstruction::Fail { .. }
            ) {
                for (name, action, operand, span) in &cleanup {
                    if returned_value == provider.parameter_value(name) {
                        continue;
                    }
                    lowered.push(BackendInstruction::Cleanup {
                        action: action.clone(),
                        operand: Some(*operand),
                        span: *span,
                    });
                }
            }
            lowered.push(instruction);
        }
        block.instructions = lowered;
    }
}

fn parameter_cleanup(
    function: &super::BackendFunction,
    provider: &SharedExpressionProvider<'_>,
) -> Vec<ParameterCleanup> {
    let mut cleanup = Vec::new();
    for plan in function
        .cleanup
        .iter()
        .filter(|plan| !matches!(plan.origin, super::BackendCleanupOrigin::Explicit { .. }))
    {
        for (index, action) in plan.actions.iter().enumerate() {
            let super::BackendCleanupAction::DropBinding { name, .. } = action else {
                continue;
            };
            let Some(value) = provider.parameter_value(name) else { continue };
            let Some(descriptor) = plan.resolved_bridges.get(index).and_then(|items| items.first())
            else {
                continue;
            };
            if cleanup.iter().any(|(existing, _, _, _)| existing == name) {
                continue;
            }
            cleanup.push((name.clone(), action.clone(), descriptor.operand(value), plan.span));
        }
    }
    cleanup
}

#[cfg(test)]
mod tests;
