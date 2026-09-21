use std::collections::HashMap;

use cranelift_codegen::ir::AbiParam;
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::{ExternalVerbDecl, VerbDecl};

use super::native::{FunctionMeta, NativeEmitError};
use super::types::NativeType;

pub(super) fn declare_functions(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    entry_symbol: &str,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let mut metadata = HashMap::new();
    for verb in verbs {
        let signature = native_signature_for_definition(module, verb);
        let symbol = if verb.name == entry_symbol { entry_symbol } else { &verb.name };
        let id = module
            .declare_function(symbol, Linkage::Export, &signature)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        metadata
            .insert(verb.name.clone(), function_meta(id, &verb.params, verb.return_type.as_ref()));
    }
    for verb in external_verbs {
        let signature = external_native_signature(module, verb);
        let id = module
            .declare_function(&verb.name, Linkage::Import, &signature)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        metadata
            .insert(verb.name.clone(), function_meta(id, &verb.params, verb.return_type.as_ref()));
    }
    Ok(metadata)
}

fn function_meta(
    id: cranelift_module::FuncId,
    params: &[crate::ast::Param],
    return_type: Option<&crate::ast::TypeName>,
) -> FunctionMeta {
    FunctionMeta {
        id,
        parameter_names: params.iter().map(|param| param.name.clone()).collect(),
        return_type: NativeType::from_type_name(return_type),
    }
}

pub(super) fn native_signature_for_definition(
    module: &mut ObjectModule,
    verb: &VerbDecl,
) -> cranelift_codegen::ir::Signature {
    signature_for(module, &verb.params, verb.return_type.as_ref())
}

fn external_native_signature(
    module: &mut ObjectModule,
    verb: &ExternalVerbDecl,
) -> cranelift_codegen::ir::Signature {
    signature_for(module, &verb.params, verb.return_type.as_ref())
}

fn signature_for(
    module: &mut ObjectModule,
    params: &[crate::ast::Param],
    return_type: Option<&crate::ast::TypeName>,
) -> cranelift_codegen::ir::Signature {
    let mut signature = module.make_signature();
    let pointer_type = module.isa().pointer_type();
    signature.params.extend(params.iter().map(|parameter| {
        AbiParam::new(NativeType::from_name(&parameter.ty.name).unwrap().ir_type(pointer_type))
    }));
    signature
        .returns
        .push(AbiParam::new(NativeType::from_type_name(return_type).ir_type(pointer_type)));
    signature
}
