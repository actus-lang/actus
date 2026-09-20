use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_native::builder as native_builder;
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ast::{Expr, Program, Stmt, TopLevelDecl};
use crate::semantic::analyze;

#[derive(Debug)]
pub struct NativeEmitError(String);

impl std::fmt::Display for NativeEmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeEmitError {}

pub fn emit_zero_return_object(symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    emit_i32_object(symbol, 0)
}

pub fn emit_program_object(program: &Program, symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    analyze(program)
        .map_err(|error| NativeEmitError(format!("semantic analysis failed: {error:?}")))?;
    let TopLevelDecl::Verb(verb) = program
        .declarations
        .first()
        .ok_or_else(|| NativeEmitError("program has no verb declarations".to_owned()))?;
    if !verb.params.is_empty() {
        return Err(NativeEmitError("native integer slice does not support parameters".to_owned()));
    }
    let value = verb
        .body
        .statements
        .iter()
        .find_map(|statement| match statement {
            Stmt::Return { value: Some(Expr::Integer { value, .. }), .. } => {
                value.parse::<i64>().ok()
            }
            _ => None,
        })
        .ok_or_else(|| {
            NativeEmitError("native integer slice requires `return <integer>;`".to_owned())
        })?;
    emit_i32_object(symbol, value)
}

fn emit_i32_object(symbol: &str, value: i64) -> Result<Vec<u8>, NativeEmitError> {
    let mut flag_builder = settings::builder();
    flag_builder.set("is_pic", "true").map_err(|error| NativeEmitError(error.to_string()))?;
    let flags = settings::Flags::new(flag_builder);
    let isa = native_builder()
        .map_err(|error| NativeEmitError(error.to_string()))?
        .finish(flags)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, "actus", default_libcall_names())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let mut module = ObjectModule::new(builder);
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
