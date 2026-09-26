use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use crate::ast::VerbDecl;

use super::declarations::native_signature_for_definition;
use super::layout::LayoutRegistry;
use super::literals::{StringDataIds, declare_string_values};
use super::lowering::lower_body;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionMeta, FunctionRef, NativeEmitError};
use super::types::NativeType;
use super::vtable::{VtableDataIds, declare_vtable_values};

#[allow(clippy::too_many_arguments)]
pub(super) fn define_function(
    module: &mut ObjectModule,
    frontend_config: TargetFrontendConfig,
    verb: &VerbDecl,
    metadata: &FunctionMeta,
    functions: &HashMap<String, FunctionMeta>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataIds,
    layouts: &LayoutRegistry,
    vtable_data: &VtableDataIds,
) -> Result<(), NativeEmitError> {
    let mut context = module.make_context();
    context.func.signature = native_signature_for_definition(module, verb, layouts);
    let references = declare_function_refs(module, &mut context.func, functions)?;
    let mut string_values = declare_string_values(module, &mut context.func, string_data);
    string_values.extend(declare_vtable_values(module, &mut context.func, vtable_data));
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut function = FunctionBuilder::new(&mut context.func, &mut function_context);
        let block = function.create_block();
        function.switch_to_block(block);
        function.append_block_params_for_function_params(block);
        let parameters = function.block_params(block).to_vec();
        let (locals, local_types) =
            bind_parameters(&mut function, module, verb, &parameters, layouts);
        function.seal_block(block);
        let flow = lower_body(
            &mut function,
            &verb.body.statements,
            &locals,
            &local_types,
            &references,
            cleanup_schedule,
            &string_values,
            layouts,
        )?;
        emit_flow(&mut function, verb.return_type.is_some(), flow)?;
        function.finalize(frontend_config);
    }
    module
        .define_function(metadata.id, &mut context)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

fn emit_flow(
    function: &mut FunctionBuilder<'_>,
    returns_value: bool,
    flow: super::lowering::Flow,
) -> Result<(), NativeEmitError> {
    match (returns_value, flow) {
        (true, super::lowering::Flow::Return(result)) => {
            function.ins().return_(&[result]);
        }
        (false, super::lowering::Flow::Fallthrough) => {
            function.ins().return_(&[]);
        }
        (true, super::lowering::Flow::Fallthrough) => {
            return Err(NativeEmitError("native function requires a return value".to_owned()));
        }
        (false, super::lowering::Flow::Return(_)) => {
            return Err(NativeEmitError("void native function returned a value".to_owned()));
        }
        (_, super::lowering::Flow::Break | super::lowering::Flow::Continue) => {
            return Err(NativeEmitError(
                "loop control escaped its loop during native lowering".to_owned(),
            ));
        }
    };
    Ok(())
}

fn bind_parameters<'a>(
    function: &mut FunctionBuilder<'_>,
    module: &ObjectModule,
    verb: &'a VerbDecl,
    parameters: &[cranelift_codegen::ir::Value],
    layouts: &LayoutRegistry,
) -> (HashMap<&'a String, cranelift_codegen::ir::Value>, HashMap<&'a String, NativeType>) {
    let mut locals = HashMap::new();
    let mut parameter_index = 0;
    for parameter in &verb.params {
        let value = parameters[parameter_index];
        parameter_index += 1;
        let local = if parameter.dispatch == crate::ast::DispatchMode::Dynamic {
            let vtable = parameters[parameter_index];
            parameter_index += 1;
            let pointer_bytes = module.isa().pointer_type().bytes();
            let slot =
                function.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    pointer_bytes * 2,
                    pointer_bytes.trailing_zeros() as u8,
                ));
            let address = function.ins().stack_addr(module.isa().pointer_type(), slot, 0);
            function.ins().store(MemFlagsData::new(), value, address, 0);
            function.ins().store(MemFlagsData::new(), vtable, address, pointer_bytes as i32);
            address
        } else {
            value
        };
        locals.insert(&parameter.name, local);
    }
    let local_types = verb
        .params
        .iter()
        .map(|parameter| {
            let ty = if parameter.dispatch == crate::ast::DispatchMode::Dynamic {
                NativeType::FatPointer
            } else {
                NativeType::from_type_name_with_layout(Some(&parameter.ty), layouts)
            };
            (&parameter.name, ty)
        })
        .collect();
    (locals, local_types)
}

fn declare_function_refs(
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
                },
            ))
        })
        .collect()
}
