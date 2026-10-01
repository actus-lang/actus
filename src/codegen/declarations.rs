use std::collections::HashMap;

use cranelift_codegen::ir::AbiParam;
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::{DispatchMode, ExternalVerbDecl, VerbDecl};

use super::layout::LayoutRegistry;
use super::native::{FunctionMeta, NativeEmitError, NativeSymbolBindings};
use super::symbols::{SymbolIdentity, SymbolKind};
use super::types::NativeType;

pub(super) struct FunctionDeclarationContext<'a> {
    pub(super) entry_symbol: Option<&'a str>,
    pub(super) namespace_prefix: &'a str,
    pub(super) layouts: &'a LayoutRegistry,
    pub(super) bindings: &'a NativeSymbolBindings,
    pub(super) force_entry_return: bool,
}

pub(super) fn declare_functions(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    context: FunctionDeclarationContext<'_>,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let mut metadata = HashMap::new();
    declare_internal_functions(module, verbs, &context, &mut metadata)?;
    declare_external_functions(
        module,
        external_verbs,
        context.layouts,
        context.bindings,
        &mut metadata,
    )?;
    Ok(metadata)
}

fn declare_internal_functions(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    context: &FunctionDeclarationContext<'_>,
    metadata: &mut HashMap<String, FunctionMeta>,
) -> Result<(), NativeEmitError> {
    for verb in verbs {
        let signature = native_signature_for_entry(
            module,
            verb,
            context.layouts,
            context.force_entry_return && context.entry_symbol == Some(verb.name.as_str()),
        )?;
        let symbol = internal_symbol(&verb.name, context.entry_symbol, context.namespace_prefix)?;
        let id = module
            .declare_function(&symbol, Linkage::Export, &signature)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        let return_type = verb.return_type.as_ref().map(|return_type| &return_type.ty);
        metadata.insert(
            verb.name.clone(),
            function_meta(id, &verb.params, return_type, context.layouts)?,
        );
    }
    Ok(())
}

fn declare_external_functions(
    module: &mut ObjectModule,
    verbs: &[&ExternalVerbDecl],
    layouts: &LayoutRegistry,
    bindings: &NativeSymbolBindings,
    metadata: &mut HashMap<String, FunctionMeta>,
) -> Result<(), NativeEmitError> {
    for verb in verbs {
        let signature = external_native_signature(module, verb, layouts)?;
        let id = module
            .declare_function(bindings.external_symbol(&verb.name), Linkage::Import, &signature)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        let return_type = verb.return_type.as_ref().map(|return_type| &return_type.ty);
        metadata.insert(verb.name.clone(), function_meta(id, &verb.params, return_type, layouts)?);
    }
    Ok(())
}

fn internal_symbol(
    name: &str,
    entry_symbol: Option<&str>,
    namespace_prefix: &str,
) -> Result<String, NativeEmitError> {
    if entry_symbol == Some(name) {
        return Ok(name.to_owned());
    }
    SymbolIdentity::new(namespace_prefix, SymbolKind::Verb, name)
        .map(|identity| identity.as_str().to_owned())
        .map_err(|error| NativeEmitError(error.to_string()))
}

fn function_meta(
    id: cranelift_module::FuncId,
    params: &[crate::ast::Param],
    return_type: Option<&crate::ast::TypeName>,
    layouts: &LayoutRegistry,
) -> Result<FunctionMeta, NativeEmitError> {
    let return_type = NativeType::from_type_name_with_layout(return_type, layouts)?;
    Ok(FunctionMeta {
        id,
        parameter_names: params.iter().map(|param| param.name.clone()).collect(),
        return_type,
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
        ins_params: params
            .iter()
            .map(|parameter| parameter_uses_indirect_ins(parameter, layouts))
            .collect(),
    })
}

fn parameter_uses_indirect_ins(parameter: &crate::ast::Param, layouts: &LayoutRegistry) -> bool {
    parameter.role == crate::ast::Role::Ins
        && NativeType::try_from_type_name_with_layout(Some(&parameter.ty), layouts)
            .is_some_and(NativeType::uses_indirect_ins)
}

pub(super) fn native_signature_for_definition(
    module: &mut ObjectModule,
    verb: &VerbDecl,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Signature, NativeEmitError> {
    signature_for(
        module,
        &verb.params,
        verb.return_type.as_ref().map(|return_type| &return_type.ty),
        layouts,
        false,
    )
}

pub(super) fn native_signature_for_entry(
    module: &mut ObjectModule,
    verb: &VerbDecl,
    layouts: &LayoutRegistry,
    force_return: bool,
) -> Result<cranelift_codegen::ir::Signature, NativeEmitError> {
    signature_for(
        module,
        &verb.params,
        verb.return_type.as_ref().map(|return_type| &return_type.ty),
        layouts,
        force_return,
    )
}

fn external_native_signature(
    module: &mut ObjectModule,
    verb: &ExternalVerbDecl,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Signature, NativeEmitError> {
    signature_for(
        module,
        &verb.params,
        verb.return_type.as_ref().map(|return_type| &return_type.ty),
        layouts,
        false,
    )
}

fn signature_for(
    module: &mut ObjectModule,
    params: &[crate::ast::Param],
    return_type: Option<&crate::ast::TypeName>,
    layouts: &LayoutRegistry,
    force_return: bool,
) -> Result<cranelift_codegen::ir::Signature, NativeEmitError> {
    let mut signature = module.make_signature();
    let pointer_type = module.isa().pointer_type();
    let native_return = NativeType::from_type_name_with_layout(return_type, layouts)?;
    let uses_return_slot = layouts.uses_return_slot(native_return);
    if uses_return_slot {
        signature.params.push(AbiParam::new(pointer_type));
    }
    for parameter in params {
        append_parameter(&mut signature, parameter, pointer_type, layouts)?;
    }
    append_return(
        &mut signature,
        return_type,
        native_return,
        uses_return_slot,
        layouts,
        force_return,
    )?;
    Ok(signature)
}

fn append_parameter(
    signature: &mut cranelift_codegen::ir::Signature,
    parameter: &crate::ast::Param,
    pointer_type: cranelift_codegen::ir::Type,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let native_type = if parameter.dispatch == DispatchMode::Dynamic {
        NativeType::FatPointer
    } else {
        NativeType::from_type_name_with_layout(Some(&parameter.ty), layouts)?
    };
    let parameter_type =
        if parameter_uses_indirect_ins(parameter, layouts) || native_type.is_wide_integer() {
            pointer_type
        } else {
            layouts.ir_type(native_type)?
        };
    signature.params.push(AbiParam::new(parameter_type));
    if parameter.dispatch == DispatchMode::Dynamic {
        signature.params.push(AbiParam::new(pointer_type));
    }
    Ok(())
}

fn append_return(
    signature: &mut cranelift_codegen::ir::Signature,
    return_type: Option<&crate::ast::TypeName>,
    native_return: NativeType,
    uses_return_slot: bool,
    layouts: &LayoutRegistry,
    force_return: bool,
) -> Result<(), NativeEmitError> {
    let returns_value = return_type.is_some()
        && (!uses_return_slot || layouts.returns_borrowed_view(native_return))
        && (!matches!(native_return, NativeType::Void) || force_return);
    if returns_value {
        let return_type = if force_return { NativeType::Int } else { native_return };
        signature.returns.push(AbiParam::new(layouts.ir_type(return_type)?));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::internal_symbol;

    #[test]
    fn keeps_only_the_entry_symbol_public() {
        assert_eq!(internal_symbol("main", Some("main"), "actus_root").unwrap(), "main");
        assert_eq!(
            internal_symbol("read", Some("main"), "actus_root").unwrap(),
            "actus_root__verb_read"
        );
    }
}
