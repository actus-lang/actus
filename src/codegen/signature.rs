use crate::ast::{Param, PrimitiveType, ReturnAccess, ReturnType, Role, TypeName, VerbDecl};

use super::{
    BackendContractError, BackendFunction, BackendLayout, BackendParameter, BackendPassingMode,
    BackendReturn, BackendReturnMode, BackendTargetContext, BackendValueType, SymbolIdentity,
    SymbolKind,
};

pub trait BackendLayoutProvider {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendSignature {
    pub symbol: SymbolIdentity,
    pub parameters: Vec<BackendParameter>,
    pub return_value: BackendReturn,
}

impl BackendSignature {
    pub fn into_function(
        self,
        entry: super::BackendBlockId,
        blocks: Vec<super::BackendBlock>,
    ) -> BackendFunction {
        BackendFunction {
            symbol: self.symbol,
            parameters: self.parameters,
            return_value: self.return_value,
            entry,
            blocks,
        }
    }
}

pub fn lower_verb_signature(
    namespace: &str,
    verb: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendSignature, BackendContractError> {
    let symbol = SymbolIdentity::new(namespace, SymbolKind::Verb, &verb.name)
        .map_err(|error| unsupported(verb.span, error.to_string()))?;
    let parameters = verb
        .params
        .iter()
        .map(|parameter| lower_parameter(parameter, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let return_value = lower_return(verb.return_type.as_ref(), layouts)?;
    Ok(BackendSignature { symbol, parameters, return_value })
}

fn lower_parameter(
    parameter: &Param,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendParameter, BackendContractError> {
    let (ty, layout) = lower_type(&parameter.ty, layouts)?;
    let passing = passing_for(parameter.role.clone(), layout, parameter.span)?;
    Ok(BackendParameter { name: parameter.name.clone(), role: parameter.role.clone(), ty, passing })
}

fn lower_return(
    return_type: Option<&ReturnType>,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendReturn, BackendContractError> {
    let Some(return_type) = return_type else {
        return Ok(BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        });
    };
    let (ty, layout) = lower_type(&return_type.ty, layouts)?;
    let passing =
        layout.map_or(BackendReturnMode::Direct, |layout| BackendReturnMode::ReturnSlot { layout });
    let role = Some(match return_type.access {
        ReturnAccess::Owned => Role::Erg,
        ReturnAccess::Abs => Role::Abs,
    });
    Ok(BackendReturn { role, ty, passing })
}

fn lower_type(
    type_name: &TypeName,
    layouts: &impl BackendLayoutProvider,
) -> Result<(BackendValueType, Option<BackendLayout>), BackendContractError> {
    let scalar = match type_name.name.as_str() {
        "Bool" => Some(BackendValueType::Bool),
        "Int" => Some(BackendValueType::Integer { signed: true, width: 32 }),
        "Usize" => Some(BackendValueType::Integer { signed: false, width: 64 }),
        _ => match crate::ast::primitive_type(&type_name.name) {
            Some(PrimitiveType::Integer { signed, width }) => {
                Some(BackendValueType::Integer { signed, width })
            }
            Some(PrimitiveType::Float { width }) => Some(BackendValueType::Float { width }),
            Some(PrimitiveType::Void) => Some(BackendValueType::Unit),
            None => None,
        },
    };
    if let Some(scalar) = scalar {
        return Ok((scalar, None));
    }
    let layout = layouts.layout_for(type_name).ok_or_else(|| {
        unsupported(
            type_name.span,
            format!("no neutral layout is registered for `{}`", type_name.name),
        )
    })?;
    let ty = (type_name.name == "Region")
        .then_some(BackendValueType::Region { layout })
        .unwrap_or(BackendValueType::Aggregate { layout });
    Ok((ty, Some(layout)))
}

fn passing_for(
    role: Role,
    layout: Option<BackendLayout>,
    span: crate::lexer::SourceSpan,
) -> Result<BackendPassingMode, BackendContractError> {
    match (role, layout) {
        (Role::Erg, Some(layout)) => Ok(BackendPassingMode::Indirect { layout }),
        (Role::Erg, None) | (Role::Abs, None) => Ok(BackendPassingMode::Direct),
        (Role::Abs, Some(layout)) => Ok(BackendPassingMode::Borrowed { layout }),
        (Role::Ins, Some(layout)) => Ok(BackendPassingMode::ExclusiveLoan { layout }),
        (Role::Dat, Some(layout)) => Ok(BackendPassingMode::OwnershipTransfer { layout }),
        (Role::Ins | Role::Dat, None) => {
            Err(unsupported(span, "exclusive or transferred values require a layout"))
        }
    }
}

fn unsupported(span: crate::lexer::SourceSpan, reason: impl Into<String>) -> BackendContractError {
    BackendContractError::Unsupported(super::BackendUnsupportedOperation::new(
        "semantic-signature",
        span,
        BackendTargetContext {
            backend: "neutral-contract".to_owned(),
            target: "semantic-model".to_owned(),
        },
        reason,
    ))
}
