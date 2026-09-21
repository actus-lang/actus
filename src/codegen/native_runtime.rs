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
    let allocate_spec = IntrinsicKind::Allocate.spec();
    let append_spec = IntrinsicKind::Append.spec();
    let print_int_spec = IntrinsicKind::PrintInt.spec();

    Ok(HashMap::from([
        (
            allocate_spec.name.to_owned(),
            FunctionMeta {
                id: allocate_id,
                parameter_names: allocate_spec
                    .parameters
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
                return_type: NativeType::Buffer,
            },
        ),
        (
            "actus_buffer_drop".to_owned(),
            FunctionMeta {
                id: drop_id,
                parameter_names: vec!["handle".to_owned()],
                return_type: NativeType::Int,
            },
        ),
        (
            append_spec.name.to_owned(),
            FunctionMeta {
                id: append_id,
                parameter_names: append_spec
                    .parameters
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
                return_type: NativeType::Int,
            },
        ),
        (
            print_int_spec.name.to_owned(),
            FunctionMeta {
                id: print_int_id,
                parameter_names: print_int_spec
                    .parameters
                    .iter()
                    .map(|name| (*name).to_owned())
                    .collect(),
                return_type: NativeType::Int,
            },
        ),
    ]))
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
