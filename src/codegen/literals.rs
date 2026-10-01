use std::collections::{HashMap, HashSet};

use cranelift_codegen::ir::GlobalValue;
use cranelift_module::{DataDescription, DataId, Linkage, Module};
use cranelift_object::ObjectModule;

use crate::ast::{Expr, Stmt, VerbDecl};

use super::symbols::{SymbolIdentity, SymbolKind};

pub(super) type StringDataIds = HashMap<String, DataId>;

pub(super) struct StringDataValues {
    values: HashMap<String, GlobalValue>,
    namespace_prefix: String,
}

impl StringDataValues {
    pub(super) fn get(&self, key: &str) -> Option<&GlobalValue> {
        self.values.get(key)
    }

    pub(super) fn extend_vtables(&mut self, values: HashMap<String, GlobalValue>) {
        self.values.extend(values);
    }

    pub(super) fn vtable_symbol(
        &self,
        role: &str,
        native_type: crate::codegen::types::NativeType,
    ) -> String {
        super::vtable::vtable_symbol_for_native_in_namespace(
            &self.namespace_prefix,
            role,
            native_type,
        )
    }
}

pub(super) fn define_string_data(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    namespace_prefix: &str,
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
            let symbol =
                SymbolIdentity::new(namespace_prefix, SymbolKind::Data, &format!("string_{index}"))
                    .map_err(|error| error.to_string())?;
            let data_id = module
                .declare_data(symbol.as_str(), Linkage::Local, false, false)
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
    namespace_prefix: &str,
) -> StringDataValues {
    let values = data_ids
        .iter()
        .map(|(value, data_id)| (value.clone(), module.declare_data_in_func(*data_id, function)))
        .collect();
    StringDataValues { values, namespace_prefix: namespace_prefix.to_owned() }
}

fn collect_block(statements: &[Stmt], values: &mut HashSet<String>) {
    for statement in statements {
        match statement {
            Stmt::OwnerDecl { initializer, .. }
            | Stmt::Expression { expression: initializer, .. } => {
                collect_expression(initializer, values)
            }
            Stmt::Assignment { value, .. } => collect_expression(value, values),
            Stmt::FieldAssignment { object, value, .. } => {
                collect_expression(object, values);
                collect_expression(value, values);
            }
            Stmt::IndexAssignment { target, index, value, .. } => {
                collect_expression(target, values);
                collect_expression(index, values);
                collect_expression(value, values);
            }
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
        | Expr::Cast { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. } => collect_expression(expression, values),
        Expr::Binary { left, right, .. } => {
            collect_expression(left, values);
            collect_expression(right, values);
        }
        Expr::Call { arguments, .. } => collect_arguments(arguments, values),
        Expr::MethodCall { receiver, arguments, .. } => {
            collect_expression(receiver, values);
            collect_arguments(arguments, values);
        }
        Expr::StructLit { fields, .. } => collect_struct_fields(fields, values),
        Expr::FieldAccess { object, .. } => collect_expression(object, values),
        Expr::Index { target, index, .. } => {
            collect_expression(target, values);
            collect_expression(index, values);
        }
        Expr::Case { subject, branches, .. } => collect_case(subject, branches, values),
        Expr::If { condition, then_branch, else_branch, .. } => {
            collect_expression(condition, values);
            collect_block(&then_branch.statements, values);
            if let Some(crate::ast::IfBranch::Block(block)) = else_branch {
                collect_block(&block.statements, values);
            }
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. } => {}
    }
}

fn collect_struct_fields(fields: &[crate::ast::StructFieldInit], values: &mut HashSet<String>) {
    for field in fields {
        collect_expression(&field.value, values);
    }
}

fn collect_arguments(arguments: &[crate::ast::Argument], values: &mut HashSet<String>) {
    for argument in arguments {
        collect_expression(&argument.expression, values);
    }
}

fn collect_case(subject: &Expr, branches: &[crate::ast::CaseBranch], values: &mut HashSet<String>) {
    collect_expression(subject, values);
    for branch in branches {
        if let crate::ast::CaseBody::Expression(expression) = &branch.body {
            collect_expression(expression, values);
        }
    }
}
