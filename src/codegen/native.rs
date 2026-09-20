use std::collections::HashMap;

use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};
use cranelift_native::builder as native_builder;
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ast::{BinaryOp, Expr, Program, Role, Stmt, TopLevelDecl, VerbDecl};
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
    emit_verb_object(symbol, verb)
}

fn emit_verb_object(symbol: &str, verb: &VerbDecl) -> Result<Vec<u8>, NativeEmitError> {
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
        let result = lower_body(&mut function, &verb.body.statements)?;
        function.ins().return_(&[result]);
        function.finalize(frontend_config);
    }
    module
        .define_function(function_id, &mut context)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    module.finish().emit().map_err(|error| NativeEmitError(error.to_string()))
}

fn lower_body(
    function: &mut cranelift_frontend::FunctionBuilder<'_>,
    statements: &[Stmt],
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let mut locals = HashMap::new();
    for statement in statements {
        match statement {
            Stmt::OwnerDecl { role: Role::Erg, name, initializer, .. } => {
                let value = lower_expression(function, initializer, &locals)?;
                locals.insert(name, value);
            }
            Stmt::Assignment { name, value, .. } => {
                let value = lower_expression(function, value, &locals)?;
                locals.insert(name, value);
            }
            Stmt::Return { value: Some(expression), .. } => {
                return lower_expression(function, expression, &locals);
            }
            Stmt::Return { value: None, .. } => {
                return Err(NativeEmitError(
                    "native integer slice requires a return value".to_owned(),
                ));
            }
            _ => {
                return Err(NativeEmitError(
                    "native integer slice supports only integer declarations, assignments, and returns"
                        .to_owned(),
                ));
            }
        }
    }
    Err(NativeEmitError("native integer slice requires a return value".to_owned()))
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

fn lower_expression(
    function: &mut cranelift_frontend::FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Integer { value, .. } => value
            .parse::<i64>()
            .map(|value| function.ins().iconst(types::I32, value))
            .map_err(|error| NativeEmitError(format!("invalid integer literal: {error}"))),
        Expr::Identifier { name, .. } => locals
            .get(name)
            .copied()
            .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable"))),
        Expr::Binary { left, operator, right, .. } => {
            let left = lower_expression(function, left, locals)?;
            let right = lower_expression(function, right, locals)?;
            let value = match operator {
                BinaryOp::Add => function.ins().iadd(left, right),
                BinaryOp::Subtract => function.ins().isub(left, right),
                BinaryOp::Multiply => function.ins().imul(left, right),
                BinaryOp::Divide => function.ins().sdiv(left, right),
            };
            Ok(value)
        }
        _ => Err(NativeEmitError(
            "native integer slice supports only integer expressions".to_owned(),
        )),
    }
}
