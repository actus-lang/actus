use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Role, VerbDecl};

use super::super::BackendPassingMode;
use super::{aggregate, state};
use crate::codegen::{BackendValueId, BackendValueType};

pub(super) struct ProviderParameterState {
    pub(super) parameters: BTreeMap<String, BackendValueId>,
    pub(super) scalar_loan_parameters:
        Vec<(String, BackendValueId, BackendValueId, BackendValueType)>,
    pub(super) aggregate_loan_parameters:
        BTreeMap<String, (BackendValueId, crate::codegen::BackendLayout)>,
    pub(super) owned_parameters: BTreeSet<String>,
    pub(super) value_types: BTreeMap<BackendValueId, BackendValueType>,
    pub(super) nominal_types: BTreeMap<BackendValueId, crate::ast::TypeName>,
    pub(super) enum_payload_types: BTreeMap<(BackendValueId, u32), BackendValueType>,
    pub(super) next_value: u32,
}

pub(super) fn initialize(
    verb: &VerbDecl,
    layouts: &impl super::super::super::BackendLayoutProvider,
) -> ProviderParameterState {
    let mut parameters = verb
        .params
        .iter()
        .enumerate()
        .map(|(index, parameter)| (parameter.name.clone(), BackendValueId(index as u32)))
        .collect::<BTreeMap<_, _>>();
    let owned_parameters = verb
        .params
        .iter()
        .filter(|parameter| parameter.role == Role::Erg)
        .map(|parameter| parameter.name.clone())
        .collect();
    let mut value_types = state::parameter_types(verb, layouts);
    let mut nominal_types = aggregate::parameter_nominal_types(verb, layouts);
    for (index, parameter) in verb.params.iter().enumerate() {
        nominal_types.insert(BackendValueId(index as u32), parameter.ty.clone());
    }
    let enum_payload_types = state::enum_payload_types(verb, layouts);
    let mut scalar_loan_parameters = Vec::new();
    let mut aggregate_loan_parameters = BTreeMap::new();
    let mut next_value = verb.params.len() as u32;
    for (index, parameter) in verb.params.iter().enumerate() {
        register_aggregate_loan(parameter, index, layouts, &mut aggregate_loan_parameters);
        let Some(BackendValueType::Bool | BackendValueType::Integer { .. }) =
            value_types.get(&BackendValueId(index as u32)).copied()
        else {
            continue;
        };
        if !is_scalar_loan(parameter, layouts) {
            continue;
        }
        apply_scalar_loan(
            parameter,
            BackendValueId(index as u32),
            &mut next_value,
            &mut parameters,
            &mut value_types,
            &mut nominal_types,
            &mut scalar_loan_parameters,
        );
    }
    ProviderParameterState {
        parameters,
        scalar_loan_parameters,
        aggregate_loan_parameters,
        owned_parameters,
        value_types,
        nominal_types,
        enum_payload_types,
        next_value,
    }
}

fn register_aggregate_loan(
    parameter: &crate::ast::Param,
    index: usize,
    layouts: &impl super::super::super::BackendLayoutProvider,
    aggregate_loan_parameters: &mut BTreeMap<
        String,
        (BackendValueId, crate::codegen::BackendLayout),
    >,
) {
    if let Some(crate::codegen::BackendPassingMode::ExclusiveLoan { layout }) =
        crate::codegen::lower_parameter(parameter, layouts).ok().map(|parameter| parameter.passing)
    {
        aggregate_loan_parameters
            .insert(parameter.name.clone(), (BackendValueId(index as u32), layout));
    }
}

fn is_scalar_loan(
    parameter: &crate::ast::Param,
    layouts: &impl super::super::super::BackendLayoutProvider,
) -> bool {
    matches!(
        crate::codegen::lower_parameter(parameter, layouts).ok().map(|parameter| parameter.passing),
        Some(BackendPassingMode::ExclusiveScalarLoan { .. })
    )
}

fn apply_scalar_loan(
    parameter: &crate::ast::Param,
    address: BackendValueId,
    next_value: &mut u32,
    parameters: &mut BTreeMap<String, BackendValueId>,
    value_types: &mut BTreeMap<BackendValueId, BackendValueType>,
    nominal_types: &mut BTreeMap<BackendValueId, crate::ast::TypeName>,
    scalar_loan_parameters: &mut Vec<(String, BackendValueId, BackendValueId, BackendValueType)>,
) {
    let loaded = BackendValueId(*next_value);
    *next_value += 1;
    let ty = value_types[&address];
    parameters.insert(parameter.name.clone(), loaded);
    value_types.insert(loaded, ty);
    if let Some(type_name) = nominal_types.get(&address).cloned() {
        nominal_types.insert(loaded, type_name);
    }
    scalar_loan_parameters.push((parameter.name.clone(), address, loaded, ty));
}
