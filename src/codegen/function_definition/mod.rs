mod flow;
mod parameters;

use std::collections::HashMap;

use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use crate::ast::VerbDecl;

use super::declarations::{native_signature_for_definition, native_signature_for_entry};
use super::layout::LayoutRegistry;
use super::literals::{StringDataIds, StringDataValues, declare_string_values};
use super::lowering::lower_body;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionMeta, FunctionRef, NativeEmitError};
use super::types::NativeType;
use super::vtable::{VtableDataIds, declare_vtable_values};
use flow::emit_flow;
use parameters::{BoundParameters, bind_parameters, declare_function_refs};

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
    namespace_prefix: &str,
    force_entry_return: bool,
) -> Result<(), NativeEmitError> {
    let mut context = module.make_context();
    context.func.signature = if force_entry_return {
        native_signature_for_entry(module, verb, layouts, true)?
    } else {
        native_signature_for_definition(module, verb, layouts)?
    };
    lower_function_body(
        module,
        &mut context,
        frontend_config,
        verb,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
        vtable_data,
        namespace_prefix,
        force_entry_return,
    )?;
    module
        .define_function(metadata.id, &mut context)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_function_body(
    module: &mut ObjectModule,
    context: &mut cranelift_codegen::Context,
    frontend_config: TargetFrontendConfig,
    verb: &VerbDecl,
    functions: &HashMap<String, FunctionMeta>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataIds,
    layouts: &LayoutRegistry,
    vtable_data: &VtableDataIds,
    namespace_prefix: &str,
    force_entry_return: bool,
) -> Result<(), NativeEmitError> {
    let references = declare_function_refs(module, &mut context.func, functions)?;
    let mut string_values =
        declare_string_values(module, &mut context.func, string_data, namespace_prefix);
    string_values.extend_vtables(declare_vtable_values(module, &mut context.func, vtable_data));
    let mut function_context = FunctionBuilderContext::new();
    let mut function = FunctionBuilder::new(&mut context.func, &mut function_context);
    let (return_type, return_slot, flow) = lower_function_flow(
        &mut function,
        module,
        verb,
        &references,
        cleanup_schedule,
        &string_values,
        layouts,
        force_entry_return,
    )?;
    emit_flow(&mut function, return_type, return_slot, flow, layouts, force_entry_return)?;
    function.finalize(frontend_config);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_function_flow(
    function: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    verb: &VerbDecl,
    references: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_values: &StringDataValues,
    layouts: &LayoutRegistry,
    force_entry_return: bool,
) -> Result<
    (Option<NativeType>, Option<cranelift_codegen::ir::Value>, super::lowering::Flow),
    NativeEmitError,
> {
    let (return_type, return_slot, (locals, local_types)) =
        setup_function_entry(function, module, verb, layouts, force_entry_return)?;
    let flow = lower_body(
        function,
        &verb.body.statements,
        &locals,
        &local_types,
        references,
        cleanup_schedule,
        string_values,
        layouts,
    )?;
    Ok((return_type, return_slot, flow))
}

fn setup_function_entry<'a>(
    function: &mut FunctionBuilder<'_>,
    module: &mut ObjectModule,
    verb: &'a VerbDecl,
    layouts: &LayoutRegistry,
    force_entry_return: bool,
) -> Result<
    (Option<NativeType>, Option<cranelift_codegen::ir::Value>, BoundParameters<'a>),
    NativeEmitError,
> {
    let block = function.create_block();
    function.switch_to_block(block);
    function.append_block_params_for_function_params(block);
    let parameters = function.block_params(block).to_vec();
    let return_type = declared_return_type(verb, layouts)?.map(|return_type| {
        if force_entry_return && matches!(return_type, NativeType::Void) {
            NativeType::Int
        } else {
            return_type
        }
    });
    let return_slot = return_type.filter(|ty| layouts.uses_return_slot(*ty)).map(|_| parameters[0]);
    let parameter_offset = usize::from(return_slot.is_some());
    let (locals, local_types) =
        bind_parameters(function, module, verb, &parameters[parameter_offset..], layouts)?;
    function.seal_block(block);
    Ok((return_type, return_slot, (locals, local_types)))
}

fn declared_return_type(
    verb: &VerbDecl,
    layouts: &LayoutRegistry,
) -> Result<Option<NativeType>, NativeEmitError> {
    verb.return_type
        .as_ref()
        .map(|return_type| NativeType::from_type_name_with_layout(Some(&return_type.ty), layouts))
        .transpose()
}
