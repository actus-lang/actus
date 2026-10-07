use std::collections::HashMap;

use cranelift_codegen::ir::{AbiParam, types};
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::IntrinsicKind;
use crate::runtime::{
    BUFFER_ALLOCATE_SYMBOL, BUFFER_APPEND_SYMBOL, BUFFER_CRC32_MATCHES_SYMBOL, BUFFER_CRC32_SYMBOL,
    BUFFER_DROP_SYMBOL, BUFFER_LENGTH_SYMBOL, BUFFER_VALIDATE_FIXED_FRAME_SYMBOL,
    ENUM_ALLOCATE_SYMBOL, ENUM_DROP_SYMBOL, PRINT_BUFFER_STDOUT_SYMBOL, PRINT_INT_SYMBOL,
    PRINT_STRING_SYMBOL, REGION_DROP_SYMBOL, WRITE_STRING_STDOUT_SYMBOL,
};

use super::native::{FunctionMeta, NativeEmitError};
use super::types::NativeType;

const SERIALIZATION_I64: NativeType = NativeType::Integer { signed: true, width: 64 };
const VALIDATE_FIXED_FRAME_PARAMS: &[&str] = &[
    "buffer",
    "little",
    "version_offset",
    "expected_version",
    "payload_offset",
    "payload_length",
    "checksum_start",
    "checksum_end",
    "checksum_offset",
];

pub(super) fn declare_runtime_functions(
    module: &mut ObjectModule,
    has_print_definition: bool,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let pointer_type = module.isa().pointer_type();
    let ids = RuntimeFunctionIds {
        allocate: declare_allocate(module, pointer_type)?,
        drop: declare_drop(module, pointer_type)?,
        region_drop: declare_region_drop(module)?,
        append: declare_append(module, pointer_type)?,
        buffer_length: declare_buffer_length(module, pointer_type)?,
        crc32: declare_crc32(module, pointer_type)?,
        crc32_matches: declare_crc32_matches(module, pointer_type)?,
        validate_fixed_frame: declare_validate_fixed_frame(module, pointer_type)?,
        print_int: declare_print_int(module)?,
        print_string: declare_print_string(module, pointer_type)?,
        write_string: declare_write_string(module, pointer_type)?,
        enum_allocate: declare_enum_allocate(module, pointer_type)?,
        enum_drop: declare_enum_drop(module, pointer_type)?,
    };
    declare_print_buffer(module, pointer_type)?;
    let append_spec = IntrinsicKind::Append.spec();
    let print_spec = IntrinsicKind::Print.spec();
    let mut functions = runtime_metadata(&ids, &append_spec, &print_spec);
    if has_print_definition {
        functions.remove(print_spec.name);
    }
    Ok(functions)
}

pub(super) fn declare_region_cleanup_function(
    module: &mut ObjectModule,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let region_drop = declare_region_drop(module)?;
    Ok(HashMap::from([(
        super::performance::dispatch_key(NativeType::Region, "drop"),
        named_meta(region_drop, &["handle"], NativeType::Int),
    )]))
}

pub(super) fn declare_enum_cleanup_function(
    module: &mut ObjectModule,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let enum_drop = declare_enum_drop(module, module.isa().pointer_type())?;
    Ok(HashMap::from([(
        ENUM_DROP_SYMBOL.to_owned(),
        named_meta(enum_drop, &["pointer", "size"], NativeType::Int),
    )]))
}

pub(super) fn declare_buffer_cleanup_function(
    module: &mut ObjectModule,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let buffer_drop = declare_drop(module, module.isa().pointer_type())?;
    Ok(HashMap::from([(
        BUFFER_DROP_SYMBOL.to_owned(),
        named_meta(buffer_drop, &["handle"], NativeType::Int),
    )]))
}

struct RuntimeFunctionIds {
    allocate: cranelift_module::FuncId,
    drop: cranelift_module::FuncId,
    region_drop: cranelift_module::FuncId,
    append: cranelift_module::FuncId,
    buffer_length: cranelift_module::FuncId,
    crc32: cranelift_module::FuncId,
    crc32_matches: cranelift_module::FuncId,
    validate_fixed_frame: cranelift_module::FuncId,
    print_int: cranelift_module::FuncId,
    print_string: cranelift_module::FuncId,
    write_string: cranelift_module::FuncId,
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
            super::performance::dispatch_key(NativeType::Region, "drop"),
            named_meta(ids.region_drop, &["handle"], NativeType::Int),
        ),
        ("buffer_length".to_owned(), named_meta(ids.buffer_length, &["buffer"], NativeType::Int)),
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
        ("crc32".to_owned(), named_meta(ids.crc32, &["buffer", "start", "end"], SERIALIZATION_I64)),
        (
            "crc32_matches".to_owned(),
            named_meta(
                ids.crc32_matches,
                &["buffer", "start", "end", "expected"],
                SERIALIZATION_I64,
            ),
        ),
        (
            "validate_fixed_frame".to_owned(),
            named_meta(ids.validate_fixed_frame, VALIDATE_FIXED_FRAME_PARAMS, SERIALIZATION_I64),
        ),
        (
            print_spec.name.to_owned(),
            intrinsic_meta(ids.print_int, print_spec.parameters, NativeType::Int),
        ),
        (PRINT_STRING_SYMBOL.to_owned(), named_meta(ids.print_string, &["text"], NativeType::Int)),
        (
            WRITE_STRING_STDOUT_SYMBOL.to_owned(),
            named_meta(ids.write_string, &["text"], NativeType::Int),
        ),
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

fn declare_region_drop(
    module: &mut ObjectModule,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I32));
    module
        .declare_function(REGION_DROP_SYMBOL, Linkage::Import, &signature)
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

fn declare_buffer_length(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));
    module
        .declare_function(BUFFER_LENGTH_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_crc32(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(types::I64));
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I64));
    module
        .declare_function(BUFFER_CRC32_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_crc32_matches(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.params.push(AbiParam::new(types::I64));
    signature.params.push(AbiParam::new(types::I64));
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I64));
    module
        .declare_function(BUFFER_CRC32_MATCHES_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_validate_fixed_frame(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    for _ in 0..8 {
        signature.params.push(AbiParam::new(types::I64));
    }
    signature.returns.push(AbiParam::new(types::I64));
    module
        .declare_function(BUFFER_VALIDATE_FIXED_FRAME_SYMBOL, Linkage::Import, &signature)
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

fn declare_print_buffer(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));
    module
        .declare_function(PRINT_BUFFER_STDOUT_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_write_string(
    module: &mut ObjectModule,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<cranelift_module::FuncId, NativeEmitError> {
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(pointer_type));
    signature.returns.push(AbiParam::new(types::I32));
    module
        .declare_function(WRITE_STRING_STDOUT_SYMBOL, Linkage::Import, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))
}
