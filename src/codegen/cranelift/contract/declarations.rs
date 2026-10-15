use std::collections::BTreeMap;

use cranelift_codegen::ir::{AbiParam, Type, types};
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use super::super::aggregate::{
    Declaration, declaration, direct_aggregate_types, runtime_declaration,
};
use super::super::native::BackendEmitError;
use crate::codegen::{BackendModule, BackendPassingMode, BackendValueType};

pub(super) fn declare_functions(
    module: &mut ObjectModule,
    backend_module: &BackendModule,
) -> Result<BTreeMap<String, Declaration>, BackendEmitError> {
    let mut declarations = BTreeMap::new();
    for external in &backend_module.externals {
        let signature = signature(
            module,
            &external.parameters,
            &external.return_value,
            false,
            external.runtime_pointer_return,
        )?;
        let id = module
            .declare_function(external.symbol.as_str(), Linkage::Import, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        declarations.insert(
            external.symbol.as_str().to_owned(),
            runtime_declaration(module, id, external)?,
        );
    }
    for function in &backend_module.functions {
        let signature = signature(
            module,
            &function.parameters,
            &function.return_value,
            function.hosted_entry_return,
            false,
        )?;
        let id = module
            .declare_function(function.symbol.as_str(), Linkage::Export, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        let lowered = declaration(module, id, &function.parameters, &function.return_value)?;
        declarations.insert(function.symbol.as_str().to_owned(), lowered);
    }
    super::cleanup::declare_cleanup_bridges(module, backend_module, &mut declarations)?;
    Ok(declarations)
}

pub(super) fn signature(
    module: &ObjectModule,
    parameters: &[crate::codegen::BackendParameter],
    return_value: &crate::codegen::BackendReturn,
    hosted_entry_return: bool,
    runtime_pointer_return: bool,
) -> Result<cranelift_codegen::ir::Signature, BackendEmitError> {
    let mut signature = module.make_signature();
    for parameter in parameters {
        for ty in parameter_types(module, parameter)? {
            signature.params.push(AbiParam::new(ty));
        }
    }
    match &return_value.passing {
        crate::codegen::BackendReturnMode::Direct => {
            if hosted_entry_return {
                signature.returns.push(AbiParam::new(types::I32));
            } else if let Some(ty) = scalar_type(module, return_value.ty)? {
                signature.returns.push(AbiParam::new(ty));
            }
        }
        crate::codegen::BackendReturnMode::DirectAggregate { abi } => {
            for ty in direct_aggregate_types(module, abi, false)? {
                signature.returns.push(AbiParam::new(ty));
            }
        }
        crate::codegen::BackendReturnMode::ReturnSlot { .. } if runtime_pointer_return => {
            signature.returns.push(AbiParam::new(module.isa().pointer_type()));
            if matches!(
                return_value.ty,
                crate::codegen::BackendValueType::NichePointerOption { .. }
            ) {
                signature.params.insert(0, AbiParam::new(module.isa().pointer_type()));
            }
        }
        crate::codegen::BackendReturnMode::ReturnSlot { layout } => {
            if matches!(return_value.ty, BackendValueType::NichePointerOption { .. }) {
                signature.params.insert(0, AbiParam::new(module.isa().pointer_type()));
                return Ok(signature);
            }
            if !matches!(return_value.ty, BackendValueType::Aggregate { layout: value_layout } | BackendValueType::Region { layout: value_layout } | BackendValueType::Enum { layout: crate::codegen::BackendEnumLayout { layout: value_layout, .. } } if value_layout == *layout)
            {
                return Err(BackendEmitError(
                    "Cranelift return slot layout does not match the aggregate return type"
                        .to_owned(),
                ));
            }
            signature.params.insert(0, AbiParam::new(module.isa().pointer_type()));
        }
    }
    Ok(signature)
}

pub(crate) fn parameter_types(
    module: &ObjectModule,
    parameter: &crate::codegen::BackendParameter,
) -> Result<Vec<Type>, BackendEmitError> {
    match &parameter.passing {
        BackendPassingMode::Direct => {
            scalar_type(module, parameter.ty)?.map(|ty| vec![ty]).ok_or_else(|| {
                BackendEmitError("Cranelift direct unit parameters are not supported".to_owned())
            })
        }
        BackendPassingMode::DirectAggregate { abi } => direct_aggregate_types(module, abi, true),
        BackendPassingMode::Indirect { layout }
        | BackendPassingMode::Borrowed { layout }
        | BackendPassingMode::ExclusiveLoan { layout }
        | BackendPassingMode::OwnershipTransfer { layout }
        | BackendPassingMode::RuntimeBridge { layout } => {
            layout.validate().map_err(|error| BackendEmitError(error.to_string()))?;
            match parameter.ty {
                BackendValueType::Pointer => Ok(vec![module.isa().pointer_type()]),
                BackendValueType::Aggregate { layout: value_layout }
                | BackendValueType::Region { layout: value_layout }
                | BackendValueType::Enum {
                    layout: crate::codegen::BackendEnumLayout { layout: value_layout, .. },
                } if value_layout == *layout => Ok(vec![module.isa().pointer_type()]),
                BackendValueType::Aggregate { .. }
                | BackendValueType::Region { .. }
                | BackendValueType::Enum { .. }
                | BackendValueType::NichePointerOption { .. } => Err(BackendEmitError(
                    "Cranelift aggregate parameter layout does not match passing layout".to_owned(),
                )),
                _ => Err(BackendEmitError(
                    "Cranelift indirect parameter must have an aggregate or Region type".to_owned(),
                )),
            }
        }
        BackendPassingMode::ExclusiveScalarLoan { .. } => Ok(vec![module.isa().pointer_type()]),
    }
}

pub(crate) fn scalar_type(
    module: &ObjectModule,
    ty: BackendValueType,
) -> Result<Option<Type>, BackendEmitError> {
    let result = match ty {
        BackendValueType::Unit => None,
        BackendValueType::Bool => Some(types::I8),
        BackendValueType::Integer { width: 8, .. } => Some(types::I8),
        BackendValueType::Integer { width: 16, .. } => Some(types::I16),
        BackendValueType::Integer { width: 32, .. } => Some(types::I32),
        BackendValueType::Integer { width: 64, .. } => Some(types::I64),
        BackendValueType::Pointer => Some(module.isa().pointer_type()),
        BackendValueType::Region { .. } => Some(module.isa().pointer_type()),
        BackendValueType::Enum { .. } => Some(module.isa().pointer_type()),
        BackendValueType::NichePointerOption { .. } => Some(module.isa().pointer_type()),
        BackendValueType::Float { width: 32 } => Some(types::F32),
        BackendValueType::Float { width: 64 } => Some(types::F64),
        BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Aggregate { .. } => {
            return Err(BackendEmitError(
                "Cranelift neutral contract slice supports scalar values only".to_owned(),
            ));
        }
    };
    Ok(result)
}
