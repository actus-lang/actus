use crate::ast::{
    Argument, BuiltinType, DispatchMode, ExternalVerbDecl, GenericParam, ReturnAccess, Role,
    VerbDecl, lookup_builtin_type,
};

use super::super::analyzer::canonical_type_name;

#[derive(Clone)]
pub(crate) struct VerbSignature {
    pub(crate) params: Vec<(String, Role, String)>,
    pub(crate) dynamic_params: Vec<DispatchMode>,
    pub(crate) return_type: Option<BuiltinType>,
    pub(crate) return_type_name: Option<crate::ast::TypeName>,
    pub(crate) return_access: Option<ReturnAccess>,
    pub(crate) generic_parameters: Vec<GenericParam>,
    pub(crate) external: bool,
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
        }
    }
}
