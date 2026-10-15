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

impl<'program> SharedDeclarations<'program> {
    pub(crate) fn new(
        namespace: &str,
        program: &'program Program,
        layouts: &impl BackendLayoutProvider,
        performance_context: Option<(&str, &'program Program)>,
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
        let context_performances = performance_context
            .map(|(_, context)| scalar_performances(context))
            .unwrap_or_default();
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
        let external_signatures = externals
            .iter()
            .map(|external| lower_external_signature(external, layouts))
            .collect::<Result<Vec<_>, _>>()?;
        let mut return_types = shared_return_types(
            &verbs,
            &externals,
            &signatures,
            &external_signatures,
            &performances,
            &performance_signatures,
        );
        for ((_, method), signature) in
            context_performances.iter().zip(&context_performance_signatures)
        {
            return_types.insert(
                method.name.clone(),
                (signature.return_value.ty != BackendValueType::Unit)
                    .then_some(signature.return_value.ty),
            );
        }
        let mut return_payload_types =
            shared_return_payload_types(&verbs, &externals, layouts, &performances);
        return_payload_types
            .extend(shared_return_payload_types_for_performances(&context_performances, layouts));
        let mut return_nominal_types =
            performance::shared_return_nominal_types(&verbs, &externals, &performances);
        return_nominal_types.extend(context_performances.iter().map(|(_, method)| {
            (
                method.name.clone(),
                method.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            )
        }));
        let (
            mut performance_symbols,
            mut performance_return_types,
            mut performance_return_nominal_types,
        ) = performance::performance_dispatch_maps(&performances, &performance_signatures);
        let (context_symbols, context_return_types, context_return_nominal_types) =
            performance::performance_dispatch_maps(
                &context_performances,
                &context_performance_signatures,
            );
        performance_symbols.extend(context_symbols);
        performance_return_types.extend(context_return_types);
        performance_return_nominal_types.extend(context_return_nominal_types);
        let performance_externals = context_performances
            .iter()
            .zip(context_performance_signatures.iter())
            .map(|((_, _), signature)| performance_external(signature))
            .collect::<Result<Vec<_>, _>>()?;
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
            performance_externals,
        })
    }
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
