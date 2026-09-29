use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use crate::ast::{DispatchMode, Param, VerbDecl};

use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionMeta, FunctionRef, NativeEmitError};
use super::super::types::NativeType;

pub(super) type BoundParameters<'a> =
    (HashMap<&'a String, cranelift_codegen::ir::Value>, HashMap<&'a String, NativeType>);

pub(super) fn bind_parameters<'a>(
    function: &mut FunctionBuilder<'_>,
    module: &ObjectModule,
    verb: &'a VerbDecl,
    parameters: &[cranelift_codegen::ir::Value],
    layouts: &LayoutRegistry,
) -> Result<BoundParameters<'a>, NativeEmitError> {
    let mut locals = HashMap::new();
    let mut parameter_index = 0;
    for parameter in &verb.params {
        let (local, next_index) =
            bind_parameter(function, module, parameter, parameters, parameter_index);
        parameter_index = next_index;
        locals.insert(&parameter.name, local);
    }
    let local_types = parameter_types(verb, layouts)?;
    Ok((locals, local_types))
}

fn bind_parameter(
    function: &mut FunctionBuilder<'_>,
    module: &ObjectModule,
    parameter: &Param,
    parameters: &[cranelift_codegen::ir::Value],
    parameter_index: usize,
) -> (cranelift_codegen::ir::Value, usize) {
    let value = parameters[parameter_index];
    if parameter.dispatch != DispatchMode::Dynamic {
        return (value, parameter_index + 1);
    }
    let vtable = parameters[parameter_index + 1];
    let pointer_bytes = module.isa().pointer_type().bytes();
    let slot = function.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
        pointer_bytes * 2,
        pointer_bytes.trailing_zeros() as u8,
    ));
    let address = function.ins().stack_addr(module.isa().pointer_type(), slot, 0);
    function.ins().store(MemFlagsData::new(), value, address, 0);
    function.ins().store(MemFlagsData::new(), vtable, address, pointer_bytes as i32);
    (address, parameter_index + 2)
}

fn parameter_types<'a>(
    verb: &'a VerbDecl,
    layouts: &LayoutRegistry,
) -> Result<HashMap<&'a String, NativeType>, NativeEmitError> {
    verb.params
        .iter()
        .map(|parameter| {
            let ty = if parameter.dispatch == DispatchMode::Dynamic {
                Ok(NativeType::FatPointer)
            } else {
                NativeType::from_type_name_with_layout(Some(&parameter.ty), layouts)
            };
            ty.map(|ty| (&parameter.name, ty))
        })
        .collect()
}

pub(super) fn declare_function_refs(
    module: &mut ObjectModule,
    function: &mut cranelift_codegen::ir::Function,
    functions: &HashMap<String, FunctionMeta>,
) -> Result<HashMap<String, FunctionRef>, NativeEmitError> {
    functions
        .iter()
        .map(|(name, meta)| {
            let reference = module.declare_func_in_func(meta.id, function);
            Ok((
                name.clone(),
                FunctionRef {
                    reference,
                    parameter_names: meta.parameter_names.clone(),
                    return_type: meta.return_type,
                    dynamic_params: meta.dynamic_params.clone(),
                    dynamic_roles: meta.dynamic_roles.clone(),
                    ins_params: meta.ins_params.clone(),
                },
            ))
        })
        .collect()
}
