use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, PerformDecl, Program, TopLevelDecl, TypeName, VerbDecl};

use super::performance;
use super::support::unsupported;
use super::{
    BackendContractError, BackendExternal, BackendLayoutProvider, BackendPassingMode,
    BackendSignature, BackendSymbolBindings, BackendValueType, SymbolIdentity, SymbolKind,
    lower_parameter, lower_return,
};
use crate::codegen::lower_verb_signature;

pub(crate) struct SharedDeclarations<'program> {
    pub(crate) verbs: Vec<&'program VerbDecl>,
    pub(crate) externals: Vec<&'program ExternalVerbDecl>,
    pub(crate) performances: Vec<(&'program PerformDecl, &'program VerbDecl)>,
    pub(crate) performance_signatures: Vec<BackendSignature>,
    pub(crate) external_signatures: Vec<BackendSignature>,
    pub(crate) return_types: BTreeMap<String, Option<BackendValueType>>,
    pub(crate) return_nominal_types: BTreeMap<String, Option<TypeName>>,
    pub(crate) return_payload_types: BTreeMap<String, [Option<BackendValueType>; 2]>,
    pub(crate) performance_symbols: BTreeMap<(String, String), SymbolIdentity>,
    pub(crate) performance_return_types: BTreeMap<(String, String), Option<BackendValueType>>,
    pub(crate) performance_return_nominal_types: BTreeMap<(String, String), Option<TypeName>>,
    pub(crate) parameter_modes: BTreeMap<String, Vec<BackendPassingMode>>,
    pub(crate) parameter_types: BTreeMap<String, Vec<BackendValueType>>,
    pub(crate) performance_externals: Vec<BackendExternal>,
}

struct DeclarationInputs<'program> {
    verbs: Vec<&'program VerbDecl>,
    externals: Vec<&'program ExternalVerbDecl>,
    performances: Vec<(&'program PerformDecl, &'program VerbDecl)>,
    signatures: Vec<BackendSignature>,
    performance_signatures: Vec<BackendSignature>,
    external_signatures: Vec<BackendSignature>,
    context_performances: Vec<(&'program PerformDecl, &'program VerbDecl)>,
    context_performance_signatures: Vec<BackendSignature>,
}

type ReturnMetadata = (
    BTreeMap<String, Option<BackendValueType>>,
    BTreeMap<String, Option<TypeName>>,
    BTreeMap<String, [Option<BackendValueType>; 2]>,
);

type PerformanceMetadata = (
    BTreeMap<(String, String), SymbolIdentity>,
    BTreeMap<(String, String), Option<BackendValueType>>,
    BTreeMap<(String, String), Option<TypeName>>,
    Vec<BackendExternal>,
);

impl<'program> SharedDeclarations<'program> {
    pub(crate) fn new(
        namespace: &str,
        program: &'program Program,
        layouts: &impl BackendLayoutProvider,
        performance_context: Option<(&str, &'program Program)>,
    ) -> Result<Self, BackendContractError> {
        let inputs = collect_declaration_inputs(namespace, program, layouts, performance_context)?;
        let (return_types, return_nominal_types, return_payload_types) =
            build_return_metadata(&inputs, layouts);
        let (
            performance_symbols,
            performance_return_types,
            performance_return_nominal_types,
            performance_externals,
        ) = build_performance_metadata(&inputs)?;
        let (parameter_modes, parameter_types) = build_parameter_metadata(&inputs);
        Ok(Self {
            verbs: inputs.verbs,
            externals: inputs.externals,
            performances: inputs.performances,
            performance_signatures: inputs.performance_signatures,
            external_signatures: inputs.external_signatures,
            return_types,
            return_nominal_types,
            return_payload_types,
            performance_symbols,
            performance_return_types,
            performance_return_nominal_types,
            parameter_modes,
            parameter_types,
            performance_externals,
        })
    }
}

fn collect_declaration_inputs<'program>(
    namespace: &str,
    program: &'program Program,
    layouts: &impl BackendLayoutProvider,
    performance_context: Option<(&str, &'program Program)>,
) -> Result<DeclarationInputs<'program>, BackendContractError> {
    let (verbs, externals) = scalar_declarations(program)?;
    let performances = scalar_performances(program);
    let performance_signatures = performances
        .iter()
        .map(|(perform, method)| lower_performance_signature(namespace, perform, method, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let signatures = verbs
        .iter()
        .map(|verb| lower_verb_signature(namespace, verb, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let external_signatures = externals
        .iter()
        .map(|external| lower_external_signature(external, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let context_performances =
        performance_context.map(|(_, context)| scalar_performances(context)).unwrap_or_default();
    let context_performance_signatures = performance_context
        .map(|(context_namespace, _)| {
            context_performances
                .iter()
                .map(|(perform, method)| {
                    lower_performance_signature(context_namespace, perform, method, layouts)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(DeclarationInputs {
        verbs,
        externals,
        performances,
        signatures,
        performance_signatures,
        external_signatures,
        context_performances,
        context_performance_signatures,
    })
}

fn build_return_metadata(
    inputs: &DeclarationInputs<'_>,
    layouts: &impl BackendLayoutProvider,
) -> ReturnMetadata {
    let mut return_types = shared_return_types(
        &inputs.verbs,
        &inputs.externals,
        &inputs.signatures,
        &inputs.external_signatures,
        &inputs.performances,
        &inputs.performance_signatures,
    );
    let mut return_nominal_types = performance::shared_return_nominal_types(
        &inputs.verbs,
        &inputs.externals,
        &inputs.performances,
    );
    let mut return_payload_types = shared_return_payload_types(
        &inputs.verbs,
        &inputs.externals,
        layouts,
        &inputs.performances,
    );
    for ((_, method), signature) in
        inputs.context_performances.iter().zip(&inputs.context_performance_signatures)
    {
        return_types.insert(
            method.name.clone(),
            (signature.return_value.ty != BackendValueType::Unit)
                .then_some(signature.return_value.ty),
        );
        return_nominal_types.insert(
            method.name.clone(),
            method.return_type.as_ref().map(|return_type| return_type.ty.clone()),
        );
    }
    return_payload_types.extend(shared_return_payload_types_for_performances(
        &inputs.context_performances,
        layouts,
    ));
    (return_types, return_nominal_types, return_payload_types)
}

fn build_performance_metadata(
    inputs: &DeclarationInputs<'_>,
) -> Result<PerformanceMetadata, BackendContractError> {
    let (mut symbols, mut return_types, mut nominal_types) = performance::performance_dispatch_maps(
        &inputs.performances,
        &inputs.performance_signatures,
    );
    let (context_symbols, context_return_types, context_nominal_types) =
        performance::performance_dispatch_maps(
            &inputs.context_performances,
            &inputs.context_performance_signatures,
        );
    symbols.extend(context_symbols);
    return_types.extend(context_return_types);
    nominal_types.extend(context_nominal_types);
    let imported_externals = inputs
        .performances
        .iter()
        .zip(&inputs.performance_signatures)
        .filter(|((perform, _), _)| perform.module_import)
        .map(|(_, signature)| performance_external(signature))
        .collect::<Result<Vec<_>, _>>()?;
    let context_externals = inputs
        .context_performance_signatures
        .iter()
        .map(performance_external)
        .collect::<Result<Vec<_>, _>>()?;
    let externals = imported_externals.into_iter().chain(context_externals).collect();
    Ok((symbols, return_types, nominal_types, externals))
}

fn build_parameter_metadata(
    inputs: &DeclarationInputs<'_>,
) -> (BTreeMap<String, Vec<BackendPassingMode>>, BTreeMap<String, Vec<BackendValueType>>) {
    let parameter_modes = inputs
        .verbs
        .iter()
        .zip(&inputs.signatures)
        .map(|(verb, signature)| {
            (
                verb.name.clone(),
                signature.parameters.iter().map(|parameter| parameter.passing.clone()).collect(),
            )
        })
        .chain(inputs.externals.iter().zip(&inputs.external_signatures).map(
            |(external, signature)| {
                (
                    external.name.clone(),
                    signature
                        .parameters
                        .iter()
                        .map(|parameter| parameter.passing.clone())
                        .collect(),
                )
            },
        ))
        .collect();
    let parameter_types = inputs
        .verbs
        .iter()
        .zip(&inputs.signatures)
        .map(|(verb, signature)| {
            (verb.name.clone(), signature.parameters.iter().map(|parameter| parameter.ty).collect())
        })
        .chain(inputs.externals.iter().zip(&inputs.external_signatures).map(
            |(external, signature)| {
                (
                    external.name.clone(),
                    signature.parameters.iter().map(|parameter| parameter.ty).collect(),
                )
            },
        ))
        .collect();
    (parameter_modes, parameter_types)
}

pub(crate) fn build_backend_externals(
    externals: &[&ExternalVerbDecl],
    signatures: Vec<BackendSignature>,
    symbols: &BTreeMap<String, SymbolIdentity>,
) -> Result<Vec<BackendExternal>, BackendContractError> {
    let mut unique: BTreeMap<SymbolIdentity, BackendExternal> = BTreeMap::new();
    for (external, signature) in externals.iter().zip(signatures) {
        let lowered = scalar_external(external, signature, symbols)?;
        if let Some(existing) = unique.get(&lowered.symbol) {
            if !compatible_external_signature(existing, &lowered)
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

fn compatible_external_signature(left: &BackendExternal, right: &BackendExternal) -> bool {
    left.parameters.len() == right.parameters.len()
        && left.parameters.iter().zip(&right.parameters).all(|(left, right)| {
            left.role == right.role && left.ty == right.ty && left.passing == right.passing
        })
}

fn performance_external(
    signature: &BackendSignature,
) -> Result<BackendExternal, BackendContractError> {
    Ok(BackendExternal {
        symbol: SymbolIdentity::external_raw(signature.symbol.as_str())
            .map_err(|_| BackendContractError::InvalidExternalSymbol)?,
        parameters: signature.parameters.clone(),
        return_value: signature.return_value.clone(),
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
    let symbol_namespace = perform.module_import_namespace.as_deref().unwrap_or(namespace);
    let symbol = SymbolIdentity::specialized(
        symbol_namespace,
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

pub(crate) fn scalar_external_symbols(
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

use super::returns::{
    shared_return_payload_types, shared_return_payload_types_for_performances, shared_return_types,
};
