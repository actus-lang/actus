use std::collections::HashMap;

use cranelift_codegen::ir::AbiParam;
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::{DispatchMode, ExternalVerbDecl, VerbDecl};

use super::layout::LayoutRegistry;
use super::native::{FunctionMeta, NativeEmitError};
use super::types::NativeType;

pub(super) fn declare_functions(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    entry_symbol: &str,
    layouts: &LayoutRegistry,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let mut metadata = HashMap::new();
    for verb in verbs {
        let signature = native_signature_for_definition(module, verb, layouts);
        let symbol = if verb.name == entry_symbol { entry_symbol } else { &verb.name };
        let id = module
            .declare_function(symbol, Linkage::Export, &signature)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        metadata.insert(
            verb.name.clone(),
            function_meta(id, &verb.params, verb.return_type.as_ref(), layouts),
        );
    }
    for verb in external_verbs {
        let signature = external_native_signature(module, verb, layouts);
        let id = module
            .declare_function(&verb.name, Linkage::Import, &signature)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        metadata.insert(
            verb.name.clone(),
            function_meta(id, &verb.params, verb.return_type.as_ref(), layouts),
        );
    }
    Ok(metadata)
}

fn function_meta(
    id: cranelift_module::FuncId,
    params: &[crate::ast::Param],
    return_type: Option<&crate::ast::TypeName>,
    layouts: &LayoutRegistry,
) -> FunctionMeta {
    FunctionMeta {
        id,
        parameter_names: params.iter().map(|param| param.name.clone()).collect(),
        return_type: NativeType::from_type_name_with_layout(return_type, layouts),
        dynamic_params: params
            .iter()
            .map(|parameter| parameter.dispatch == DispatchMode::Dynamic)
            .collect(),
        dynamic_roles: params
            .iter()
            .map(|parameter| {
                (parameter.dispatch == DispatchMode::Dynamic).then(|| parameter.ty.name.clone())
            })
            .collect(),
    }
}

pub(super) fn native_signature_for_definition(
    module: &mut ObjectModule,
    verb: &VerbDecl,
    layouts: &LayoutRegistry,
) -> cranelift_codegen::ir::Signature {
    signature_for(module, &verb.params, verb.return_type.as_ref(), layouts)
}

fn external_native_signature(
    module: &mut ObjectModule,
    verb: &ExternalVerbDecl,
    layouts: &LayoutRegistry,
) -> cranelift_codegen::ir::Signature {
    signature_for(module, &verb.params, verb.return_type.as_ref(), layouts)
}

fn signature_for(
    module: &mut ObjectModule,
    params: &[crate::ast::Param],
    return_type: Option<&crate::ast::TypeName>,
    layouts: &LayoutRegistry,
) -> cranelift_codegen::ir::Signature {
    let mut signature = module.make_signature();
    let pointer_type = module.isa().pointer_type();
    for parameter in params {
        let native_type = if parameter.dispatch == DispatchMode::Dynamic {
            NativeType::FatPointer
        } else {
            NativeType::from_type_name_with_layout(Some(&parameter.ty), layouts)
        };
        signature.params.push(AbiParam::new(native_type.ir_type(pointer_type)));
        if parameter.dispatch == DispatchMode::Dynamic {
            signature.params.push(AbiParam::new(pointer_type));
        }
    }
    signature.returns.push(AbiParam::new(
        NativeType::from_type_name_with_layout(return_type, layouts).ir_type(pointer_type),
    ));
    signature
}
