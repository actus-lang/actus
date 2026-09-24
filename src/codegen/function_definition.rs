use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
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
) -> Result<(), NativeEmitError> {
    let mut context = module.make_context();
    context.func.signature = native_signature_for_definition(module, verb, layouts);
    let references = declare_function_refs(module, &mut context.func, functions)?;
    let string_values = declare_string_values(module, &mut context.func, string_data);
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut function = FunctionBuilder::new(&mut context.func, &mut function_context);
        let block = function.create_block();
        function.switch_to_block(block);
        function.append_block_params_for_function_params(block);
        let parameters = function.block_params(block).to_vec();
        let mut locals = HashMap::new();
        for (parameter, value) in verb.params.iter().zip(parameters) {
            locals.insert(&parameter.name, value);
        }
        let local_types = verb
            .params
            .iter()
            .map(|parameter| {
                (
                    &parameter.name,
                    NativeType::from_name_with_layout(&parameter.ty.name, layouts).unwrap(),
                )
            })
            .collect();
        function.seal_block(block);
        let result = lower_body(
            &mut function,
            &verb.body.statements,
            &locals,
            &local_types,
            &references,
            cleanup_schedule,
            &string_values,
            layouts,
        )?;
        function.ins().return_(&[result]);
        function.finalize(frontend_config);
    }
    module
        .define_function(metadata.id, &mut context)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
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
                },
            ))
        })
        .collect()
}
