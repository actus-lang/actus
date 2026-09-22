use std::collections::{HashMap, HashSet};

use cranelift_codegen::ir::GlobalValue;
use cranelift_module::{DataDescription, DataId, Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::{Expr, Stmt, VerbDecl};

pub(super) type StringDataIds = HashMap<String, DataId>;
pub(super) type StringDataValues = HashMap<String, GlobalValue>;

pub(super) fn define_string_data(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
) -> Result<StringDataIds, String> {
    let mut values = HashSet::new();
    for verb in verbs {
        collect_block(&verb.body.statements, &mut values);
    }
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let symbol = format!("actus_string_{index}");
            let data_id = module
                .declare_data(&symbol, Linkage::Local, false, false)
                .map_err(|error| error.to_string())?;
            let mut description = DataDescription::new();
            description.define(format!("{value}\0").into_bytes().into_boxed_slice());
            module.define_data(data_id, &description).map_err(|error| error.to_string())?;
            Ok((value, data_id))
        })
        .collect()
}

pub(super) fn declare_string_values(
    module: &mut ObjectModule,
    function: &mut cranelift_codegen::ir::Function,
    data_ids: &StringDataIds,
) -> StringDataValues {
    data_ids
        .iter()
        .map(|(value, data_id)| (value.clone(), module.declare_data_in_func(*data_id, function)))
        .collect()
}

fn collect_block(statements: &[Stmt], values: &mut HashSet<String>) {
    for statement in statements {
        match statement {
            Stmt::OwnerDecl { initializer, .. }
            | Stmt::Expression { expression: initializer, .. } => {
                collect_expression(initializer, values)
            }
            Stmt::Assignment { value, .. } => collect_expression(value, values),
            Stmt::Return { value: Some(value), .. } => collect_expression(value, values),
            Stmt::Loop(block) | Stmt::Block(block) => collect_block(&block.statements, values),
            Stmt::Return { value: None, .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Drop { .. } => {}
        }
    }
}

fn collect_expression(expression: &Expr, values: &mut HashSet<String>) {
    match expression {
        Expr::StringLiteral { value, .. } => {
            values.insert(value.clone());
        }
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Borrow { expression, .. } => collect_expression(expression, values),
        Expr::Binary { left, right, .. } => {
            collect_expression(left, values);
            collect_expression(right, values);
        }
        Expr::Call { arguments, .. } => {
            for argument in arguments {
                collect_expression(&argument.expression, values);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_expression(&field.value, values);
            }
        }
        Expr::FieldAccess { object, .. } => collect_expression(object, values),
        Expr::Identifier { .. } | Expr::Integer { .. } | Expr::FloatLiteral { .. } => {}
    }
}
