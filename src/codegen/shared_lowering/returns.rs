use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, PerformDecl, TypeName, VerbDecl};

use super::support::backend_value_type;
use super::{BackendLayoutProvider, BackendSignature, BackendValueType};

pub(crate) fn shared_return_types(
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

pub(crate) fn shared_return_payload_types(
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

pub(crate) fn shared_return_payload_types_for_performances(
    performances: &[(&PerformDecl, &VerbDecl)],
    layouts: &impl BackendLayoutProvider,
) -> BTreeMap<String, [Option<BackendValueType>; 2]> {
    performances
        .iter()
        .filter_map(|(_, method)| {
            result_payload_types(
                method.return_type.as_ref().map(|return_type| &return_type.ty),
                layouts,
            )
            .map(|types| (method.name.clone(), types))
        })
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
