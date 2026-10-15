use crate::ast::{Param, PrimitiveType, ReturnAccess, ReturnType, Role, TypeName, VerbDecl};

use super::abi::BackendEnumLayout;
use super::{
    BackendBodyProvider, BackendContractError, BackendFunction, BackendLayout, BackendParameter,
    BackendPassingMode, BackendReturn, BackendReturnMode, BackendTargetContext, BackendValueType,
    SymbolIdentity, SymbolKind, lower_verb_body,
};

pub trait BackendLayoutProvider {
    fn layout_for(&self, type_name: &TypeName) -> Option<BackendLayout>;

    fn field_for(&self, _type_name: &TypeName, _field: &str) -> Option<BackendFieldLayout> {
        None
    }

    fn packed_field_for(
        &self,
        _type_name: &TypeName,
        _field: &str,
    ) -> Option<BackendPackedFieldLayout> {
        None
    }

    fn enum_layout_for(&self, _type_name: &TypeName) -> Option<BackendEnumLayout> {
        None
    }

    fn enum_variant(&self, _type_name: &TypeName, _variant: &str) -> Option<(u32, bool)> {
        None
    }

    fn enum_variant_payload_type(
        &self,
        _type_name: &TypeName,
        _variant: &str,
    ) -> Option<BackendValueType> {
        None
    }

    fn enum_variant_for_layout(
        &self,
        _layout: BackendEnumLayout,
        _variant: &str,
    ) -> Option<(u32, bool)> {
        None
    }

    fn type_for_layout(&self, _layout: BackendLayout) -> Option<TypeName> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendFieldLayout {
    pub offset: u32,
    pub layout: BackendLayout,
    pub ty: TypeName,
    pub indirect: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendPackedFieldLayout {
    pub storage: BackendLayout,
    pub bit_offset: u16,
    pub width: u8,
    pub big_endian: bool,
    pub scalar_ty: BackendValueType,
    pub ty: TypeName,
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
        self.into_function_with_cleanup(entry, blocks, Vec::new())
    }

    pub fn into_function_with_cleanup(
        self,
        entry: super::BackendBlockId,
        blocks: Vec<super::BackendBlock>,
        cleanup: Vec<super::BackendCleanupPlan>,
    ) -> BackendFunction {
        BackendFunction {
            symbol: self.symbol,
            parameters: self.parameters,
            return_value: self.return_value,
            hosted_entry_return: false,
            entry,
            blocks,
            cleanup,
        }
    }
}

pub fn lower_verb(
    namespace: &str,
    verb: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
    body_provider: &impl BackendBodyProvider,
    entry: super::BackendBlockId,
) -> Result<BackendFunction, BackendContractError> {
    let signature = lower_verb_signature(namespace, verb, layouts)?;
    let blocks = lower_verb_body(verb, body_provider)?;
    let function = signature.into_function(entry, blocks);
    function.validate()?;
    Ok(function)
}

pub fn lower_verb_with_expressions(
    namespace: &str,
    verb: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
    body_provider: &mut impl super::BackendExpressionProvider,
    entry: super::BackendBlockId,
) -> Result<BackendFunction, BackendContractError> {
    let signature = lower_verb_signature(namespace, verb, layouts)?;
    let blocks = super::lower_verb_body_with_expressions(verb, body_provider)?;
    let function = signature.into_function(entry, blocks);
    function.validate()?;
    Ok(function)
}

pub(crate) fn lower_verb_with_expressions_as(
    symbol: SymbolIdentity,
    verb: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
    body_provider: &mut impl super::BackendExpressionProvider,
    entry: super::BackendBlockId,
) -> Result<BackendFunction, BackendContractError> {
    let signature = lower_verb_signature_as(symbol, verb, layouts)?;
    let blocks = super::lower_verb_body_with_expressions(verb, body_provider)?;
    let function = signature.into_function(entry, blocks);
    function.validate()?;
    Ok(function)
}

pub fn lower_verb_with_cleanup(
    namespace: &str,
    verb: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
    body_provider: &impl BackendBodyProvider,
    entry: super::BackendBlockId,
    cleanup: Vec<super::BackendCleanupPlan>,
) -> Result<BackendFunction, BackendContractError> {
    let signature = lower_verb_signature(namespace, verb, layouts)?;
    let blocks = lower_verb_body(verb, body_provider)?;
    let function = signature.into_function_with_cleanup(entry, blocks, cleanup);
    function.validate()?;
    Ok(function)
}

pub fn lower_verb_signature(
    namespace: &str,
    verb: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendSignature, BackendContractError> {
    let symbol = SymbolIdentity::new(namespace, SymbolKind::Verb, &verb.name)
        .map_err(|error| unsupported(verb.span, error.to_string()))?;
    lower_verb_signature_as(symbol, verb, layouts)
}

fn lower_verb_signature_as(
    symbol: SymbolIdentity,
    verb: &VerbDecl,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendSignature, BackendContractError> {
    let parameters = verb
        .params
        .iter()
        .map(|parameter| lower_parameter(parameter, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let return_value = lower_return(verb.return_type.as_ref(), layouts)?;
    Ok(BackendSignature { symbol, parameters, return_value })
}

pub(crate) fn lower_parameter(
    parameter: &Param,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendParameter, BackendContractError> {
    let (ty, layout) = lower_type(&parameter.ty, layouts)?;
    let passing = passing_for(parameter.role.clone(), ty, layout, parameter.span)?;
    Ok(BackendParameter { name: parameter.name.clone(), role: parameter.role.clone(), ty, passing })
}

pub(crate) fn lower_return(
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
    let niche_payload_layout = if matches!(ty, BackendValueType::NichePointerOption { .. }) {
        return_type.ty.arguments.first().and_then(|payload| layouts.layout_for(payload))
    } else {
        None
    };
    let passing = if is_opaque_runtime_type(&return_type.ty) {
        BackendReturnMode::Direct
    } else if let BackendValueType::NichePointerOption { layout, .. } = ty {
        BackendReturnMode::ReturnSlot { layout: niche_payload_layout.unwrap_or(layout) }
    } else {
        layout.map_or(BackendReturnMode::Direct, |layout| BackendReturnMode::ReturnSlot { layout })
    };
    let role = Some(match return_type.access {
        ReturnAccess::Owned => Role::Erg,
        ReturnAccess::Abs => Role::Abs,
    });
    if let BackendValueType::NichePointerOption { layout, .. } = ty {
        return Ok(BackendReturn {
            role,
            ty: BackendValueType::NichePointerOption {
                layout,
                payload_layout: niche_payload_layout,
            },
            passing,
        });
    }
    Ok(BackendReturn { role, ty, passing })
}

fn lower_type(
    type_name: &TypeName,
    layouts: &impl BackendLayoutProvider,
) -> Result<(BackendValueType, Option<BackendLayout>), BackendContractError> {
    if type_name.name == "Region" {
        let layout = layouts.layout_for(type_name).ok_or_else(|| {
            unsupported(type_name.span, "no neutral layout is registered for `Region`")
        })?;
        return Ok((BackendValueType::Region { layout }, Some(layout)));
    }
    if is_opaque_runtime_type(type_name) {
        return Ok((BackendValueType::Pointer, layouts.layout_for(type_name)));
    }
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
    if let Some(layout) = layouts.enum_layout_for(type_name) {
        if layout.niche_pointer {
            return Ok((
                BackendValueType::NichePointerOption {
                    layout: layout.layout,
                    payload_layout: None,
                },
                None,
            ));
        }
        if layout.payload.is_none() {
            return Ok((BackendValueType::Integer { signed: false, width: 32 }, None));
        }
        return Ok((BackendValueType::Enum { layout }, Some(layout.layout)));
    }
    let layout = layouts.layout_for(type_name).ok_or_else(|| {
        unsupported(
            type_name.span,
            format!("no neutral layout is registered for `{}`", type_name.name),
        )
    })?;
    let ty = BackendValueType::Aggregate { layout };
    Ok((ty, Some(layout)))
}

fn is_opaque_runtime_type(type_name: &TypeName) -> bool {
    matches!(type_name.name.as_str(), "Arena" | "Buffer" | "String" | "Region")
}

fn passing_for(
    role: Role,
    ty: BackendValueType,
    layout: Option<BackendLayout>,
    _span: crate::lexer::SourceSpan,
) -> Result<BackendPassingMode, BackendContractError> {
    match (role, layout) {
        (Role::Erg, Some(layout)) => Ok(BackendPassingMode::Indirect { layout }),
        (Role::Erg, None) | (Role::Abs, None) => Ok(BackendPassingMode::Direct),
        (Role::Abs, Some(layout)) => Ok(BackendPassingMode::Borrowed { layout }),
        (Role::Ins, Some(layout)) => Ok(BackendPassingMode::ExclusiveLoan { layout }),
        (Role::Dat, Some(layout)) => Ok(BackendPassingMode::OwnershipTransfer { layout }),
        (Role::Ins, None) => Ok(BackendPassingMode::ExclusiveScalarLoan { ty }),
        (Role::Dat, None) => Ok(BackendPassingMode::Direct),
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
