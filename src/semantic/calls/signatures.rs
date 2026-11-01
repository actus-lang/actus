use crate::ast::{
    Argument, BuiltinType, Capability, DispatchMode, Effect, ExternalVerbDecl, GenericParam,
    ReturnAccess, Role, VerbDecl, lookup_builtin_type,
};

use super::super::analyzer::canonical_type_name;

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct VerbSignature {
    pub(crate) params: Vec<(String, Role, String)>,
    pub(crate) dynamic_params: Vec<DispatchMode>,
    pub(crate) return_type: Option<BuiltinType>,
    pub(crate) return_type_name: Option<crate::ast::TypeName>,
    pub(crate) return_access: Option<ReturnAccess>,
    pub(crate) generic_parameters: Vec<GenericParam>,
    pub(crate) external: bool,
    pub(crate) must_use: Option<String>,
    pub(crate) effects: Vec<Effect>,
    pub(crate) capabilities: Vec<Capability>,
    pub(crate) deterministic: bool,
    pub(crate) realtime: bool,
    pub(crate) bounded: Option<crate::ast::BoundedContract>,
}

pub(super) fn has_matching_named_signature(
    signature: Option<&VerbSignature>,
    arguments: &[Argument],
) -> bool {
    let Some(signature) = signature else { return false };
    arguments.len() == signature.params.len()
        && arguments.iter().all(|argument| {
            argument.name.as_ref().is_some_and(|name| {
                signature.params.iter().any(|(parameter, _, _)| parameter == name)
            })
        })
}

impl VerbDecl {
    pub(crate) fn signature(&self) -> VerbSignature {
        VerbSignature {
            params: self
                .params
                .iter()
                .map(|parameter| {
                    (
                        parameter.name.clone(),
                        parameter.role.clone(),
                        canonical_type_name(&parameter.ty),
                    )
                })
                .collect(),
            dynamic_params: self.params.iter().map(|parameter| parameter.dispatch).collect(),
            return_type: self
                .return_type
                .as_ref()
                .and_then(|return_type| lookup_builtin_type(&return_type.ty.name)),
            return_type_name: self.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            return_access: self.return_type.as_ref().map(|return_type| return_type.access),
            generic_parameters: self.generic_parameters.clone(),
            external: false,
            must_use: self.metadata.iter().find_map(|attribute| match attribute {
                crate::ast::MetaAttribute::MustUse(reason) => Some(reason.clone()),
                _ => None,
            }),
            effects: metadata_effects(&self.metadata),
            capabilities: metadata_capabilities(&self.metadata),
            deterministic: metadata_is_deterministic(&self.metadata),
            realtime: metadata_is_realtime(&self.metadata),
            bounded: metadata_bounded(&self.metadata),
        }
    }
}

impl ExternalVerbDecl {
    pub(crate) fn signature(&self) -> VerbSignature {
        VerbSignature {
            params: self
                .params
                .iter()
                .map(|parameter| {
                    (
                        parameter.name.clone(),
                        parameter.role.clone(),
                        canonical_type_name(&parameter.ty),
                    )
                })
                .collect(),
            dynamic_params: self.params.iter().map(|parameter| parameter.dispatch).collect(),
            return_type: self
                .return_type
                .as_ref()
                .and_then(|return_type| lookup_builtin_type(&return_type.ty.name)),
            return_type_name: self.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            return_access: self.return_type.as_ref().map(|return_type| return_type.access),
            generic_parameters: self.generic_parameters.clone(),
            external: true,
            must_use: self.metadata.iter().find_map(|attribute| match attribute {
                crate::ast::MetaAttribute::MustUse(reason) => Some(reason.clone()),
                _ => None,
            }),
            effects: metadata_effects(&self.metadata),
            capabilities: metadata_capabilities(&self.metadata),
            deterministic: metadata_is_deterministic(&self.metadata),
            realtime: metadata_is_realtime(&self.metadata),
            bounded: metadata_bounded(&self.metadata),
        }
    }
}

fn metadata_effects(metadata: &[crate::ast::MetaAttribute]) -> Vec<Effect> {
    metadata
        .iter()
        .filter_map(|attribute| match attribute {
            crate::ast::MetaAttribute::Effect(effect) => Some(*effect),
            _ => None,
        })
        .collect()
}

fn metadata_capabilities(metadata: &[crate::ast::MetaAttribute]) -> Vec<Capability> {
    metadata
        .iter()
        .filter_map(|attribute| match attribute {
            crate::ast::MetaAttribute::Capability(capability) => Some(*capability),
            _ => None,
        })
        .collect()
}

fn metadata_is_deterministic(metadata: &[crate::ast::MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, crate::ast::MetaAttribute::Deterministic))
}

fn metadata_is_realtime(metadata: &[crate::ast::MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, crate::ast::MetaAttribute::Realtime))
}

fn metadata_bounded(metadata: &[crate::ast::MetaAttribute]) -> Option<crate::ast::BoundedContract> {
    metadata.iter().find_map(|attribute| match attribute {
        crate::ast::MetaAttribute::Bounded(contract) => Some(contract.clone()),
        _ => None,
    })
}
