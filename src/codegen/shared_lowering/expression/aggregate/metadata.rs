use std::collections::BTreeMap;

use crate::ast::{TypeName, VerbDecl};
use crate::codegen::{BackendLayoutProvider, BackendValueId};

pub(super) fn parameter_nominal_types(
    verb: &VerbDecl,
    layouts: &dyn BackendLayoutProvider,
) -> BTreeMap<BackendValueId, TypeName> {
    verb.params
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            layouts
                .layout_for(&parameter.ty)
                .or_else(|| layouts.enum_layout_for(&parameter.ty).map(|layout| layout.layout))
                .map(|_| (BackendValueId(index as u32), parameter.ty.clone()))
        })
        .collect()
}
