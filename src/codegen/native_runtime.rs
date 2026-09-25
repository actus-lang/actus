use std::collections::HashMap;

use cranelift_codegen::ir::{AbiParam, types};
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::IntrinsicKind;

use super::native::{FunctionMeta, NativeEmitError};
use super::types::NativeType;

pub(super) fn declare_runtime_functions(
    module: &mut ObjectModule,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let pointer_type = module.isa().pointer_type();
    let allocate_id = declare_allocate(module, pointer_type)?;
    let drop_id = declare_drop(module, pointer_type)?;
    let append_id = declare_append(module, pointer_type)?;
    let print_int_id = declare_print_int(module)?;
    let print_string_id = declare_print_string(module, pointer_type)?;
    let append_spec = IntrinsicKind::Append.spec();
    let print_spec = IntrinsicKind::Print.spec();

    Ok(HashMap::from([
        (
            "__actus_buffer_allocate".to_owned(),
            named_meta(allocate_id, &["length"], NativeType::Buffer),
        ),
        ("actus_buffer_drop".to_owned(), named_meta(drop_id, &["handle"], NativeType::Int)),
        (
            append_spec.name.to_owned(),
            intrinsic_meta(append_id, append_spec.parameters, NativeType::Int),
        ),
        (
            print_spec.name.to_owned(),
            intrinsic_meta(print_int_id, print_spec.parameters, NativeType::Int),
        ),
        ("actus_print_string".to_owned(), named_meta(print_string_id, &["value"], NativeType::Int)),
    ]))
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
        .declare_function("actus_buffer_allocate", Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_drop(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    module
        .declare_function("actus_buffer_drop", Linkage::Import, &signature)
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
        .declare_function("actus_buffer_append", Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_print_int(
    module: &mut ObjectModule,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(types::I32));
    signature.returns.push(AbiParam::new(types::I32));
    module
        .declare_function("actus_print_int", Linkage::Import, &signature)
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
        .declare_function("actus_print_string", Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}
