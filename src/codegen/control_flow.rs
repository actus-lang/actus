use std::collections::HashMap;

use cranelift_codegen::ir::{BlockArg, InstBuilder};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Stmt;

use super::native::NativeEmitError;

pub(super) fn jump_with_values(
    function: &mut FunctionBuilder<'_>,
    target: cranelift_codegen::ir::Block,
    values: Vec<cranelift_codegen::ir::Value>,
) {
    let arguments = values.into_iter().map(BlockArg::Value).collect::<Vec<_>>();
    function.ins().jump(target, arguments.iter());
}

pub(super) fn assigned_outer_bindings(
    statements: &[Stmt],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) -> Vec<String> {
    let mut names = Vec::new();
    for statement in statements {
        let name = match statement {
            Stmt::Assignment { name, .. }
                if locals.keys().any(|binding| binding.as_str() == name) =>
            {
                Some(name)
            }
            Stmt::FieldAssignment { .. } => None,
            Stmt::Block(block) | Stmt::Loop(block) => {
                names.extend(assigned_outer_bindings(&block.statements, locals));
                None
            }
            _ => None,
        };
        if let Some(name) = name {
            names.push(name.clone());
        }
    }
    names.sort();
    names.dedup();
    names
}

pub(super) fn carried_values(
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    names: &[String],
) -> Result<Vec<cranelift_codegen::ir::Value>, NativeEmitError> {
    names
        .iter()
        .map(|name| {
            locals
                .iter()
                .find(|(binding, _)| binding.as_str() == name)
                .map(|(_, value)| *value)
                .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))
        })
        .collect()
}
