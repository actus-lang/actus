use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

use super::super::target::build_isa_with_optimization;
use super::NativeEmitError;

pub(super) fn create_module(
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<ObjectModule, NativeEmitError> {
    let isa = build_isa_with_optimization(
        target,
        configuration.position_independent(),
        configuration.optimization_level(),
    )
    .map_err(|error| NativeEmitError(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, configuration.module_name(), default_libcall_names())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    Ok(ObjectModule::new(builder))
}

pub(super) fn emit_i32_object(
    symbol: &str,
    value: i64,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    let mut module = create_module(configuration, target)?;
    let frontend_config = module.isa().frontend_config();
    let mut signature = module.make_signature();
    signature.returns.push(AbiParam::new(types::I32));
    let function_id = module
        .declare_function(symbol, Linkage::Export, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut function = FunctionBuilder::new(&mut context.func, &mut function_context);
        let block = function.create_block();
        function.switch_to_block(block);
        function.seal_block(block);
        let result = function.ins().iconst(types::I32, value);
        function.ins().return_(&[result]);
        function.finalize(frontend_config);
    }
    module
        .define_function(function_id, &mut context)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    module.finish().emit().map_err(|error| NativeEmitError(error.to_string()))
}
