use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, Program, TopLevelDecl, TypeName, VerbDecl};

mod expression;
mod support;

use expression::SharedExpressionProvider;
use support::unsupported;

use super::{
    BackendBlockId, BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupProvider, BackendCleanupResolvedDescriptor, BackendCleanupResult,
    BackendContractError, BackendExternal, BackendInstruction, BackendLayout,
    BackendLayoutProvider, BackendModule, BackendSignature, BackendSymbolBindings,
    BackendValueType, SymbolIdentity, lower_parameter, lower_return, lower_verb_signature,
    lower_verb_with_expressions, lower_verb_with_expressions_as,
};

struct BuiltinLayouts;

impl BackendLayoutProvider for BuiltinLayouts {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
        matches!(type_name.name.as_str(), "Arena" | "Buffer" | "Region" | "String")
            .then_some(host_pointer_layout())
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
    lower_program_to_backend_module_with_layouts(
        namespace,
        program,
        entry_name,
        bindings,
        &BuiltinLayouts,
    )
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
            &external_symbols,
        )?);
    }
    let module = BackendModule {
        externals: externals
            .iter()
            .zip(external_signatures)
            .map(|(external, signature)| scalar_external(external, signature, &external_symbols))
            .collect::<Result<_, _>>()?,
        functions,
    };
    module.validate()?;
    Ok(module)
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

fn lower_shared_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
    layouts: &impl BackendLayoutProvider,
    cleanup_plans: &[super::BackendCleanupPlan],
    return_types: &BTreeMap<String, Option<BackendValueType>>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<super::BackendFunction, BackendContractError> {
    let mut provider =
        SharedExpressionProvider::new(namespace, verb, return_types, external_symbols);
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
    for plan in &function.cleanup {
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
mod tests {
    use super::{
        BackendLayout, BackendLayoutProvider, lower_program_to_backend_module_with_layouts,
    };
    use crate::ast::TypeName;
    use crate::codegen::{
        BackendInstruction, BackendPassingMode, BackendReturnMode, BackendValueType,
    };
    use crate::lexer::scan;
    use crate::parser::parse;

    struct RegionLayouts;

    impl BackendLayoutProvider for RegionLayouts {
        fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout> {
            (type_name.name == "Region").then_some(BackendLayout { size: 32, alignment: 8 })
        }
    }

    #[test]
    fn lowers_region_signature_and_cleanup_through_shared_pipeline() {
        let source = "verb inspect(erg region: Region[u32]) { return; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("Region fixture should parse");
        let module = lower_program_to_backend_module_with_layouts(
            "actus_root",
            &program,
            None,
            None,
            &RegionLayouts,
        )
        .expect("Region signature should lower through the shared pipeline");
        let function = &module.functions[0];
        assert!(matches!(function.parameters[0].ty, BackendValueType::Region { .. }));
        assert!(matches!(function.parameters[0].passing, BackendPassingMode::Indirect { .. }));
        assert!(matches!(function.return_value.passing, BackendReturnMode::Direct));
        assert!(
            function
                .cleanup
                .iter()
                .flat_map(|plan| plan.resolved_bridges.iter().flatten())
                .any(|descriptor| descriptor.descriptor.bridge
                    == super::BackendCleanupBridge::RegionDrop)
        );
        assert!(matches!(
            function.blocks[0].instructions.as_slice(),
            [BackendInstruction::Cleanup { .. }, BackendInstruction::Return { value: None, .. }]
        ));
    }

    #[test]
    fn lowers_region_call_and_return_through_shared_pipeline() {
        let source = "verb forward(dat region: Region[u32]) -> Region[u32] { return region; } verb main(dat region: Region[u32]) -> Region[u32] { return forward(region: dat region); }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty(), "lexer errors: {errors:?}");
        let program = parse(tokens).expect("Region call fixture should parse");
        let module = lower_program_to_backend_module_with_layouts(
            "actus_root",
            &program,
            None,
            None,
            &RegionLayouts,
        )
        .expect("Region call should lower through the shared pipeline");
        let main = module
            .functions
            .iter()
            .find(|function| function.symbol.as_str().ends_with("main"))
            .expect("main function should be present");
        assert!(main.blocks.iter().flat_map(|block| block.instructions.iter()).any(
            |instruction| matches!(
                instruction,
                crate::codegen::BackendInstruction::Call { result: Some(_), .. }
            )
        ));
    }
}
