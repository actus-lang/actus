use std::collections::BTreeMap;

use crate::ast::VerbDecl;

use super::super::super::{BackendValueId, BackendValueType};
use super::super::support::backend_value_type;

pub(super) fn parameter_types(
    verb: &VerbDecl,
    layouts: &impl super::super::super::BackendLayoutProvider,
) -> BTreeMap<BackendValueId, BackendValueType> {
    verb.params
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            super::super::super::signature::lower_parameter(parameter, layouts)
                .ok()
                .map(|parameter| (BackendValueId(index as u32), parameter.ty))
        })
        .collect()
}

pub(super) fn enum_payload_types(
    verb: &VerbDecl,
    layouts: &impl super::super::super::BackendLayoutProvider,
) -> BTreeMap<(BackendValueId, u32), BackendValueType> {
    verb.params
        .iter()
        .enumerate()
        .filter(|(_, parameter)| parameter.ty.name == "Result" && parameter.ty.arguments.len() == 2)
        .flat_map(|(index, parameter)| {
            parameter.ty.arguments.iter().enumerate().filter_map(move |(discriminant, payload)| {
                backend_value_type(layouts, payload).map(|payload_type| {
                    ((BackendValueId(index as u32), discriminant as u32), payload_type)
                })
            })
        })
        .collect()
}
