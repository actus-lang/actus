use std::collections::HashMap;

use cranelift_codegen::ir::{AbiParam, types};
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::IntrinsicKind;
use crate::runtime::{
    BUFFER_ALLOCATE_SYMBOL, BUFFER_APPEND_SYMBOL, BUFFER_DROP_SYMBOL, ENUM_ALLOCATE_SYMBOL,
    ENUM_DROP_SYMBOL, PRINT_INT_SYMBOL, PRINT_STRING_SYMBOL,
};

use super::native::{FunctionMeta, NativeEmitError};
use super::types::NativeType;

pub(super) fn declare_runtime_functions(
    module: &mut ObjectModule,
    has_print_definition: bool,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let pointer_type = module.isa().pointer_type();
    let ids = RuntimeFunctionIds {
        allocate: declare_allocate(module, pointer_type)?,
        drop: declare_drop(module, pointer_type)?,
        append: declare_append(module, pointer_type)?,
        print_int: declare_print_int(module)?,
        print_string: declare_print_string(module, pointer_type)?,
        enum_allocate: declare_enum_allocate(module, pointer_type)?,
        enum_drop: declare_enum_drop(module, pointer_type)?,
    };
    let append_spec = IntrinsicKind::Append.spec();
    let print_spec = IntrinsicKind::Print.spec();
    let mut functions = runtime_metadata(&ids, &append_spec, &print_spec);
    if has_print_definition {
        functions.remove(print_spec.name);
    }
    Ok(functions)
}

struct RuntimeFunctionIds {
    allocate: cranelift_module::FuncId,
    drop: cranelift_module::FuncId,
    append: cranelift_module::FuncId,
    print_int: cranelift_module::FuncId,
    print_string: cranelift_module::FuncId,
    enum_allocate: cranelift_module::FuncId,
    enum_drop: cranelift_module::FuncId,
}

fn runtime_metadata(
    ids: &RuntimeFunctionIds,
    append_spec: &crate::ast::IntrinsicSpec,
    print_spec: &crate::ast::IntrinsicSpec,
) -> HashMap<String, FunctionMeta> {
    HashMap::from([
        (
            format!("__{BUFFER_ALLOCATE_SYMBOL}"),
            named_meta(ids.allocate, &["length"], NativeType::Buffer),
        ),
        (BUFFER_DROP_SYMBOL.to_owned(), named_meta(ids.drop, &["handle"], NativeType::Int)),
        (
            format!("__{ENUM_ALLOCATE_SYMBOL}"),
            named_meta(ids.enum_allocate, &["size"], NativeType::Buffer),
        ),
        (
            ENUM_DROP_SYMBOL.to_owned(),
            named_meta(ids.enum_drop, &["pointer", "size"], NativeType::Int),
        ),
        (
            append_spec.name.to_owned(),
            intrinsic_meta(ids.append, append_spec.parameters, NativeType::Int),
        ),
        (
            print_spec.name.to_owned(),
            intrinsic_meta(ids.print_int, print_spec.parameters, NativeType::Int),
        ),
        (PRINT_STRING_SYMBOL.to_owned(), named_meta(ids.print_string, &["value"], NativeType::Int)),
    ])
}

fn declare_enum_allocate(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(pointer_type));
    module
        .declare_function(ENUM_ALLOCATE_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_enum_drop(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(pointer_type));
    module
        .declare_function(ENUM_DROP_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn intrinsic_meta(
    id: cranelift_module::FuncId,
    parameters: &[&str],
    return_type: NativeType,
) -> FunctionMeta {
    named_meta(id, parameters, return_type)
}

fn named_meta(
    id: cranelift_module::FuncId,
    parameters: &[&str],
    return_type: NativeType,
) -> FunctionMeta {
    FunctionMeta {
        id,
        parameter_names: parameters.iter().map(|name| (*name).to_owned()).collect(),
        return_type,
        dynamic_params: vec![false; parameters.len()],
        dynamic_roles: vec![None; parameters.len()],
        ins_params: vec![false; parameters.len()],
    }
}

fn declare_allocate(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(pointer_type));
    module
        .declare_function(BUFFER_ALLOCATE_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_drop(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    module
        .declare_function(BUFFER_DROP_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_append(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(types::I8));
    signature.returns.push(AbiParam::new(types::I8));
    module
        .declare_function(BUFFER_APPEND_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_print_int(
    module: &mut ObjectModule,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(types::I32));
    signature.returns.push(AbiParam::new(types::I32));
    module
        .declare_function(PRINT_INT_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_print_string(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));
    module
        .declare_function(PRINT_STRING_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}
