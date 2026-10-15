// Shared lowering receives explicit compiler contracts from both backends.
#![allow(clippy::too_many_arguments)]

use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, PerformDecl, Program, TopLevelDecl, TypeName, VerbDecl};
use crate::target::{EntryContract, TargetSpec};

mod constructors;
mod expression;
mod layout;
mod performance;
mod runtime_externals;
mod support;

use expression::SharedExpressionProvider;
use layout::ProgramLayouts;
use performance::lower_shared_functions;
use support::{backend_value_type, unsupported};

use super::{
    BackendBlockId, BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupProvider, BackendCleanupResolvedDescriptor,
    BackendCleanupResult, BackendContractError, BackendExternal, BackendFunction,
    BackendInstruction, BackendLayout, BackendLayoutProvider, BackendModule, BackendParameter,
    BackendPassingMode, BackendReturn, BackendReturnMode, BackendSignature, BackendSymbolBindings,
    BackendValueType, SymbolIdentity, SymbolKind, lower_parameter, lower_return,
    lower_verb_signature, lower_verb_with_expressions, lower_verb_with_expressions_as,
};

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
    let target = TargetSpec::host()
        .map_err(|error| BackendContractError::InvalidCleanupContract(error.to_string()))?;
    lower_program_to_backend_module_for_target(namespace, program, entry_name, bindings, &target)
}

pub(crate) fn lower_program_to_backend_module_for_target(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    target: &TargetSpec,
) -> Result<BackendModule, BackendContractError> {
    let layouts = ProgramLayouts::new(program, namespace);
    let semantic = crate::semantic::analyze(program).map_err(|error| {
        unsupported(
            crate::lexer::SourceSpan::new(0, 0),
            format!("semantic analysis failed: {error:?}"),
        )
    })?;
    reject_floating_point_on_freestanding(program, &semantic, target)?;
    reject_implicit_large_aggregate_allocation(&semantic, &layouts, target)?;
    let mut module = lower_program_to_backend_module_with_layouts(
        namespace, program, entry_name, bindings, &layouts,
    )?;
    for function in &mut module.functions {
        function.hosted_entry_return = target.entry_contract() == EntryContract::Hosted
            && entry_name == Some(function.symbol.as_str())
            && function.return_value.ty == BackendValueType::Unit;
    }
    module.validate()?;
    Ok(module)
}

fn reject_floating_point_on_freestanding(
    program: &Program,
    semantic: &crate::semantic::SemanticModel,
    target: &TargetSpec,
) -> Result<(), BackendContractError> {
    if target.entry_contract() == EntryContract::Hosted {
        return Ok(());
    }
    let type_name_is_float = |type_name: &TypeName| {
        matches!(
            crate::ast::primitive_type(&type_name.name),
            Some(crate::ast::PrimitiveType::Float { .. })
        )
    };
    let mut float_span =
        semantic.literal_facts.iter().find(|fact| fact.kind == "float").map(|fact| fact.span);
    if float_span.is_none() {
        for declaration in &program.declarations {
            let candidate = match declaration {
                TopLevelDecl::Verb(verb) => verb
                    .params
                    .iter()
                    .map(|parameter| &parameter.ty)
                    .chain(verb.return_type.iter().map(|return_type| &return_type.ty))
                    .find(|type_name| type_name_is_float(type_name)),
                TopLevelDecl::ExternalVerb(verb) => verb
                    .params
                    .iter()
                    .map(|parameter| &parameter.ty)
                    .chain(verb.return_type.iter().map(|return_type| &return_type.ty))
                    .find(|type_name| type_name_is_float(type_name)),
                TopLevelDecl::Struct(definition) => definition
                    .fields
                    .iter()
                    .map(|field| &field.ty)
                    .find(|type_name| type_name_is_float(type_name)),
                TopLevelDecl::Enum(definition) => {
                    definition.variants.iter().find_map(|variant| match &variant.payload {
                        crate::ast::EnumPayload::Tuple(types) => {
                            types.iter().find(|ty| type_name_is_float(ty))
                        }
                        crate::ast::EnumPayload::Struct(fields) => {
                            fields.iter().map(|field| &field.ty).find(|ty| type_name_is_float(ty))
                        }
                        crate::ast::EnumPayload::Unit => None,
                    })
                }
                _ => None,
            };
            if let Some(type_name) = candidate {
                float_span = Some(type_name.span);
                break;
            }
        }
    }
    let Some(span) = float_span else { return Ok(()) };
    Err(BackendContractError::Unsupported(super::BackendUnsupportedOperation::new(
        "floating-point lowering",
        span,
        super::BackendTargetContext {
            backend: "shared".to_owned(),
            target: target.triple().to_string(),
        },
        "floating-point values are forbidden by the freestanding target profile; use an integer or fixed-point representation",
    )))
}

fn reject_implicit_large_aggregate_allocation(
    semantic: &crate::semantic::SemanticModel,
    layouts: &ProgramLayouts<'_>,
    target: &TargetSpec,
) -> Result<(), BackendContractError> {
    if target.entry_contract() == EntryContract::Hosted {
        return Ok(());
    }
    for (binding_index, type_name) in &semantic.binding_type_names {
        let Some(layout) = layouts.layout_for(type_name) else { continue };
        if layout.size < super::LARGE_AGGREGATE_THRESHOLD {
            continue;
        }
        let span = semantic
            .bindings
            .get(*binding_index)
            .map(|binding| binding.span)
            .unwrap_or(type_name.span);
        return Err(BackendContractError::Unsupported(super::BackendUnsupportedOperation::new(
            "large aggregate implicit allocation",
            span,
            super::BackendTargetContext {
                backend: "shared".to_owned(),
                target: target.triple().to_string(),
            },
            format!(
                "aggregate layout size {} exceeds target stack limit, and implicit dynamic allocation is forbidden on freestanding targets",
                layout.size
            ),
        )));
    }
    Ok(())
}

pub(crate) fn lower_program_to_backend_module_with_layouts(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendModule, BackendContractError> {
    let declarations = SharedDeclarations::new(namespace, program, layouts)?;
    let semantic = crate::semantic::analyze(program).map_err(|error| {
        unsupported(
            crate::lexer::SourceSpan::new(0, 0),
            format!("semantic analysis failed: {error:?}"),
        )
    })?;
    let cleanup_provider = ProgramLayouts::new(program, namespace);
    let cleanup_plans = super::lower_semantic_cleanup_with_provider(&semantic, &cleanup_provider)?;
    let external_symbols = scalar_external_symbols(namespace, &declarations.externals, bindings)?;
    let functions = lower_shared_functions(
        namespace,
        entry_name,
        layouts,
        &declarations,
        &cleanup_plans,
        &external_symbols,
        bindings,
    )?;
    let mut module = BackendModule {
        externals: build_backend_externals(
            &declarations.externals,
            declarations.external_signatures,
            &external_symbols,
        )?,
        functions,
    };
    let mut runtime_symbols = referenced_runtime_symbols(&module.functions);
    if module
        .externals
        .iter()
        .any(|external| external.symbol.as_str() == crate::runtime::REGION_OPEN_SYMBOL)
    {
        runtime_symbols.insert(crate::runtime::BUFFER_DROP_SYMBOL.to_owned());
    }
    append_buffer_drop_external(&mut module.externals, &runtime_symbols)?;
    append_buffer_allocate_external(&mut module.externals, &runtime_symbols)?;
    runtime_externals::append_buffer_runtime_externals(&mut module.externals, &runtime_symbols)?;
    append_print_externals(&mut module.externals, &runtime_symbols)?;
    module.validate()?;
    Ok(module)
}

fn append_print_externals(
    externals: &mut Vec<BackendExternal>,
    required_symbols: &std::collections::BTreeSet<String>,
) -> Result<(), BackendContractError> {
    for (symbol, ty) in [
        (crate::runtime::PRINT_INT_SYMBOL, BackendValueType::Integer { signed: true, width: 32 }),
        (crate::runtime::PRINT_STRING_SYMBOL, BackendValueType::Pointer),
        (crate::runtime::WRITE_STRING_STDOUT_SYMBOL, BackendValueType::Pointer),
        (crate::runtime::PRINT_BUFFER_STDOUT_SYMBOL, BackendValueType::Pointer),
    ] {
        if !required_symbols.contains(symbol) {
            continue;
        }
        if externals.iter().any(|external| external.symbol.as_str() == symbol) {
            continue;
        }
        externals.push(BackendExternal {
            symbol: SymbolIdentity::external_raw(symbol).map_err(|error| {
                unsupported(crate::lexer::SourceSpan::new(0, 0), error.to_string())
            })?,
            parameters: vec![BackendParameter {
                name: "value".to_owned(),
                role: crate::ast::Role::Erg,
                ty,
                passing: super::BackendPassingMode::Direct,
            }],
            return_value: super::BackendReturn {
                role: None,
                ty: BackendValueType::Integer { signed: true, width: 32 },
                passing: super::BackendReturnMode::Direct,
            },
            runtime_pointer_return: false,
        });
    }
    Ok(())
}

fn referenced_runtime_symbols(functions: &[BackendFunction]) -> std::collections::BTreeSet<String> {
    let mut symbols = functions
        .iter()
        .flat_map(|function| function.blocks.iter())
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction {
            BackendInstruction::Call { callee, .. } if callee.kind() == SymbolKind::External => {
                Some(callee.as_str().to_owned())
            }
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    if symbols.contains(crate::runtime::REGION_OPEN_SYMBOL)
        || functions.iter().any(|function| {
            function.parameters.iter().any(|parameter| {
                parameter.ty == BackendValueType::Pointer
                    && matches!(parameter.passing, BackendPassingMode::OwnershipTransfer { .. })
            })
        })
    {
        symbols.insert(crate::runtime::BUFFER_DROP_SYMBOL.to_owned());
    }
    symbols
}

pub(super) struct SharedDeclarations<'program> {
    verbs: Vec<&'program VerbDecl>,
    externals: Vec<&'program ExternalVerbDecl>,
    performances: Vec<(&'program PerformDecl, &'program VerbDecl)>,
    performance_signatures: Vec<BackendSignature>,
    external_signatures: Vec<BackendSignature>,
    return_types: BTreeMap<String, Option<BackendValueType>>,
    return_nominal_types: BTreeMap<String, Option<TypeName>>,
    return_payload_types: BTreeMap<String, [Option<BackendValueType>; 2]>,
    performance_symbols: BTreeMap<(String, String), SymbolIdentity>,
    performance_return_types: BTreeMap<(String, String), Option<BackendValueType>>,
    performance_return_nominal_types: BTreeMap<(String, String), Option<TypeName>>,
    parameter_modes: BTreeMap<String, Vec<BackendPassingMode>>,
    parameter_types: BTreeMap<String, Vec<BackendValueType>>,
}

impl<'program> SharedDeclarations<'program> {
    fn new(
        namespace: &str,
        program: &'program Program,
        layouts: &impl BackendLayoutProvider,
    ) -> Result<Self, BackendContractError> {
        let (verbs, externals) = scalar_declarations(program)?;
        let performances = scalar_performances(program);
        let signatures = verbs
            .iter()
            .map(|verb| lower_verb_signature(namespace, verb, layouts))
            .collect::<Result<Vec<_>, _>>()?;
        let performance_signatures = performances
            .iter()
            .map(|(perform, method)| {
                lower_performance_signature(namespace, perform, method, layouts)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let external_signatures = externals
            .iter()
            .map(|external| lower_external_signature(external, layouts))
            .collect::<Result<Vec<_>, _>>()?;
        let return_types = shared_return_types(
            &verbs,
            &externals,
            &signatures,
            &external_signatures,
            &performances,
            &performance_signatures,
        );
        let return_payload_types =
            shared_return_payload_types(&verbs, &externals, layouts, &performances);
        let return_nominal_types =
            performance::shared_return_nominal_types(&verbs, &externals, &performances);
        let (performance_symbols, performance_return_types, performance_return_nominal_types) =
            performance::performance_dispatch_maps(&performances, &performance_signatures);
        let parameter_modes = verbs
            .iter()
            .zip(&signatures)
            .map(|(verb, signature)| {
                (
                    verb.name.clone(),
                    signature
                        .parameters
                        .iter()
                        .map(|parameter| parameter.passing.clone())
                        .collect(),
                )
            })
            .chain(externals.iter().zip(&external_signatures).map(|(external, signature)| {
                (
                    external.name.clone(),
                    signature
                        .parameters
                        .iter()
                        .map(|parameter| parameter.passing.clone())
                        .collect(),
                )
            }))
            .collect();
        let parameter_types = verbs
            .iter()
            .zip(&signatures)
            .map(|(verb, signature)| {
                (
                    verb.name.clone(),
                    signature.parameters.iter().map(|parameter| parameter.ty).collect(),
                )
            })
            .chain(externals.iter().zip(&external_signatures).map(|(external, signature)| {
                (
                    external.name.clone(),
                    signature.parameters.iter().map(|parameter| parameter.ty).collect(),
                )
            }))
            .collect();
        Ok(Self {
            verbs,
            externals,
            performances,
            performance_signatures,
            external_signatures,
            return_types,
            return_nominal_types,
            return_payload_types,
            performance_symbols,
            performance_return_types,
            performance_return_nominal_types,
            parameter_modes,
            parameter_types,
        })
    }
}

fn build_backend_externals(
    externals: &[&ExternalVerbDecl],
    signatures: Vec<BackendSignature>,
    symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<Vec<BackendExternal>, BackendContractError> {
    let mut unique: BTreeMap<SymbolIdentity, BackendExternal> = BTreeMap::new();
    for (external, signature) in externals.iter().zip(signatures) {
        let lowered = scalar_external(external, signature, symbols)?;
        if let Some(existing) = unique.get(&lowered.symbol) {
            if existing.parameters != lowered.parameters
                || existing.return_value != lowered.return_value
                || existing.runtime_pointer_return != lowered.runtime_pointer_return
            {
                return Err(BackendContractError::InvalidExternalSymbol);
            }
            continue;
        }
        unique.insert(lowered.symbol.clone(), lowered);
    }
    Ok(unique.into_values().collect())
}

fn append_buffer_allocate_external(
    externals: &mut Vec<BackendExternal>,
    required_symbols: &std::collections::BTreeSet<String>,
) -> Result<(), BackendContractError> {
    if !required_symbols.contains(crate::runtime::BUFFER_ALLOCATE_SYMBOL) {
        return Ok(());
    }
    if externals
        .iter()
        .any(|external| external.symbol.as_str() == crate::runtime::BUFFER_ALLOCATE_SYMBOL)
    {
        return Ok(());
    }
    externals.push(buffer_allocate_external()?);
    Ok(())
}

fn append_buffer_drop_external(
    externals: &mut Vec<BackendExternal>,
    required_symbols: &std::collections::BTreeSet<String>,
) -> Result<(), BackendContractError> {
    let symbol = crate::runtime::BUFFER_DROP_SYMBOL;
    if !required_symbols.contains(symbol)
        || externals.iter().any(|external| external.symbol.as_str() == symbol)
    {
        return Ok(());
    }
    externals.push(BackendExternal {
        symbol: SymbolIdentity::external_raw(symbol)
            .map_err(|error| unsupported(crate::lexer::SourceSpan::new(0, 0), error.to_string()))?,
        parameters: vec![BackendParameter {
            name: "handle".to_owned(),
            role: crate::ast::Role::Erg,
            ty: BackendValueType::Pointer,
            passing: BackendPassingMode::Direct,
        }],
        return_value: BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
        runtime_pointer_return: false,
    });
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

fn scalar_performances(program: &Program) -> Vec<(&PerformDecl, &VerbDecl)> {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Perform(perform) => Some(perform),
            _ => None,
        })
        .flat_map(|perform| perform.methods.iter().map(move |method| (perform, method)))
        .collect()
}

fn lower_performance_signature(
    namespace: &str,
    perform: &PerformDecl,
    method: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendSignature, BackendContractError> {
    let target = perform.target.canonical_key();
    let symbol = SymbolIdentity::specialized(
        namespace,
        SymbolKind::Performance,
        &method.name,
        &[&perform.role_name, &target],
    )
    .map_err(|error| unsupported(method.span, error.to_string()))?;
    let parameters = method
        .params
        .iter()
        .map(|parameter| lower_parameter(parameter, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let return_value = lower_return(method.return_type.as_ref(), layouts)?;
    Ok(BackendSignature { symbol, parameters, return_value })
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
    let runtime_pointer_return = (external.unsafe_boundary
        && !external.module_import
        && matches!(
            external.return_type.as_ref().map(|return_type| return_type.ty.name.as_str()),
            Some("Result" | "Array" | "Pack" | "Struct" | "Region")
        ))
        || matches!(signature.return_value.ty, BackendValueType::NichePointerOption { .. });
    Ok(BackendExternal {
        symbol,
        parameters: signature.parameters,
        return_value: signature.return_value,
        runtime_pointer_return,
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
    performances: &[(&PerformDecl, &VerbDecl)],
    performance_signatures: &[BackendSignature],
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
        .chain(performances.iter().zip(performance_signatures).map(|((_, method), signature)| {
            (
                method.name.clone(),
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
    performances: &[(&PerformDecl, &VerbDecl)],
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
        .chain(performances.iter().filter_map(|(_, method)| {
            result_payload_types(
                method.return_type.as_ref().map(|return_type| &return_type.ty),
                layouts,
            )
            .map(|types| (method.name.clone(), types))
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

pub(super) fn lower_shared_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
    layouts: &impl BackendLayoutProvider,
    cleanup_plans: &[super::BackendCleanupPlan],
    return_types: &BTreeMap<String, Option<BackendValueType>>,
    return_nominal_types: &BTreeMap<String, Option<TypeName>>,
    return_payload_types: &BTreeMap<String, [Option<BackendValueType>; 2]>,
    performance_symbols: &BTreeMap<(String, String), SymbolIdentity>,
    performance_return_types: &BTreeMap<(String, String), Option<BackendValueType>>,
    performance_return_nominal_types: &BTreeMap<(String, String), Option<TypeName>>,
    parameter_modes: &BTreeMap<String, Vec<BackendPassingMode>>,
    parameter_types: &BTreeMap<String, Vec<BackendValueType>>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<super::BackendFunction, BackendContractError> {
    let mut provider = SharedExpressionProvider::new(
        namespace,
        verb,
        layouts,
        return_types,
        return_nominal_types,
        return_payload_types,
        parameter_modes,
        parameter_types,
        performance_symbols,
        performance_return_types,
        performance_return_nominal_types,
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
    if let Some(entry) = function.blocks.first_mut() {
        let mut prologue = provider.scalar_loan_prologue();
        prologue.append(&mut entry.instructions);
        entry.instructions = prologue;
    }
    let writebacks = provider.scalar_loan_writebacks();
    if !writebacks.is_empty() {
        for block in &mut function.blocks {
            let Some(index) = block.instructions.iter().position(|instruction| {
                matches!(
                    instruction,
                    super::BackendInstruction::Return { .. }
                        | super::BackendInstruction::Fail { .. }
                )
            }) else {
                continue;
            };
            block.instructions.splice(index..index, writebacks.clone());
        }
    }
    function.cleanup = cleanup_plans
        .iter()
        .filter(|plan| plan.span.start >= verb.span.start && plan.span.end <= verb.span.end)
        .cloned()
        .collect();
    performance::append_parameter_cleanup(&mut function, &provider);
    function.validate()?;
    Ok(function)
}

#[cfg(test)]
mod tests;
