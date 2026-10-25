use crate::ast::{
    DispatchMode, ForeignAbi, GenericParamKind, Param, ReturnAccess, Role, TopLevelDecl,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ExternalContract {
    abi: ForeignAbi,
    unsafe_boundary: bool,
    generic_parameters: Vec<String>,
    parameters: Vec<String>,
    return_type: Option<String>,
}

pub(super) fn external_contract(declaration: &TopLevelDecl) -> Option<ExternalContract> {
    let TopLevelDecl::ExternalVerb(verb) = declaration else { return None };
    Some(ExternalContract {
        abi: verb.abi,
        unsafe_boundary: verb.unsafe_boundary,
        generic_parameters: verb.generic_parameters.iter().map(generic_parameter_key).collect(),
        parameters: verb.params.iter().map(parameter_key).collect(),
        return_type: verb.return_type.as_ref().map(|return_type| {
            format!("{}:{}", return_access_key(return_type.access), return_type.ty.canonical_key())
        }),
    })
}

fn generic_parameter_key(parameter: &crate::ast::GenericParam) -> String {
    let kind = match &parameter.kind {
        GenericParamKind::Type => "type".to_owned(),
        GenericParamKind::Const { domain } => format!("const:{}", domain.canonical_key()),
    };
    let bound =
        parameter.bound.as_ref().map(crate::ast::TypeName::canonical_key).unwrap_or_default();
    let bounds = parameter
        .bounds
        .iter()
        .map(crate::ast::TypeName::canonical_key)
        .collect::<Vec<_>>()
        .join(",");
    format!("{kind}:{bound}:{bounds}")
}

fn parameter_key(parameter: &Param) -> String {
    format!(
        "{}:{}:{}",
        role_key(&parameter.role),
        dispatch_key(parameter.dispatch),
        parameter.ty.canonical_key()
    )
}

fn role_key(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}

fn dispatch_key(dispatch: DispatchMode) -> &'static str {
    match dispatch {
        DispatchMode::Static => "static",
        DispatchMode::Dynamic => "dynamic",
    }
}

fn return_access_key(access: ReturnAccess) -> &'static str {
    match access {
        ReturnAccess::Owned => "owned",
        ReturnAccess::Abs => "abs",
    }
}
