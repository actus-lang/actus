// Shared helpers for bounded loop lowering.

use super::super::BackendValueType;
use super::super::body_contract::unsupported;
use super::BackendContractError;
use crate::ast::{Argument, Block, CaseBody, Expr, ForBinding, IfBranch, Place, Role, Stmt};
use crate::lexer::SourceSpan;

pub(super) fn range_binding_type(
    binding: &ForBinding,
    span: SourceSpan,
) -> Result<BackendValueType, BackendContractError> {
    let name = binding.ty.as_deref().unwrap_or("u32");
    let (signed, width) = match name {
        "Int" | "i8" | "i16" | "i32" | "i64" => (true, integer_width(name)),
        "u8" | "u16" | "u32" | "u64" => (false, integer_width(name)),
        _ => return Err(unsupported(span, "range binding must have an integer type")),
    };
    Ok(BackendValueType::Integer { signed, width })
}

fn integer_width(name: &str) -> u8 {
    match name {
        "i8" | "u8" => 8,
        "i16" | "u16" => 16,
        "i64" | "u64" => 64,
        _ => 32,
    }
}

pub(super) fn index_type_signed(ty: BackendValueType) -> bool {
    matches!(ty, BackendValueType::Integer { signed: true, .. })
}

pub(super) fn collect_assignment_names(block: &Block, names: &mut Vec<String>) {
    for statement in &block.statements {
        collect_statement_assignment_names(statement, names);
    }
}

pub(super) fn collect_ins_argument_names(block: &Block, names: &mut Vec<String>) {
    for statement in &block.statements {
        collect_statement_ins_argument_names(statement, names);
    }
}

fn collect_statement_ins_argument_names(statement: &Stmt, names: &mut Vec<String>) {
    match statement {
        Stmt::If { condition, then_branch, else_branch, .. } => {
            collect_expression_ins_argument_names(condition, names);
            collect_ins_argument_names(then_branch, names);
            if let Some(IfBranch::Block(block)) = else_branch {
                collect_ins_argument_names(block, names);
            }
        }
        Stmt::Loop(block) | Stmt::Block(block) => collect_ins_argument_names(block, names),
        Stmt::ForRange { start, end, body, .. } => {
            collect_expression_ins_argument_names(start, names);
            collect_expression_ins_argument_names(end, names);
            collect_ins_argument_names(body, names);
        }
        Stmt::ForArray { collection, body, .. } => {
            collect_expression_ins_argument_names(collection, names);
            collect_ins_argument_names(body, names);
        }
        Stmt::Expression { expression, .. }
        | Stmt::OwnerDecl { initializer: expression, .. }
        | Stmt::Assignment { value: expression, .. }
        | Stmt::CompoundAssignment { value: expression, .. }
        | Stmt::Return { value: Some(expression), .. } => {
            collect_expression_ins_argument_names(expression, names)
        }
        _ => {}
    }
}

fn collect_expression_ins_argument_names(expression: &Expr, names: &mut Vec<String>) {
    match expression {
        Expr::Call { arguments, .. } => collect_call_ins_argument_names(arguments, names),
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. }
        | Expr::FieldAccess { object: expression, .. } => {
            collect_expression_ins_argument_names(expression, names)
        }
        Expr::Binary { left, right, .. } => {
            collect_expression_ins_argument_names(left, names);
            collect_expression_ins_argument_names(right, names);
        }
        Expr::Index { target, index, .. } => {
            collect_expression_ins_argument_names(target, names);
            collect_expression_ins_argument_names(index, names);
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_expression_ins_argument_names(&field.value, names);
            }
        }
        Expr::Case { subject, branches, .. } => collect_case_ins_names(subject, branches, names),
        Expr::If { condition, then_branch, else_branch, .. } => {
            collect_expression_ins_argument_names(condition, names);
            collect_ins_argument_names(then_branch, names);
            if let Some(IfBranch::Block(block)) = else_branch {
                collect_ins_argument_names(block, names);
            }
        }
        Expr::MethodCall { receiver, arguments, .. } => {
            collect_expression_ins_argument_names(receiver, names);
            collect_call_ins_argument_names(arguments, names);
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => {}
    }
}

fn collect_call_ins_argument_names(arguments: &[Argument], names: &mut Vec<String>) {
    for argument in arguments {
        if argument.role == Some(Role::Ins) {
            collect_ins_place_name(argument, names);
        }
        collect_expression_ins_argument_names(&argument.expression, names);
    }
}

fn collect_case_ins_names(
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    names: &mut Vec<String>,
) {
    collect_expression_ins_argument_names(subject, names);
    for branch in branches {
        if let Some(guard) = &branch.guard {
            collect_expression_ins_argument_names(guard, names);
        }
        match &branch.body {
            CaseBody::Expression(expression) => {
                collect_expression_ins_argument_names(expression, names)
            }
            CaseBody::Block(block) => collect_ins_argument_names(block, names),
        }
    }
}

fn collect_ins_place_name(argument: &Argument, names: &mut Vec<String>) {
    if let Expr::Identifier { name, .. } = &argument.expression {
        names.push(name.clone());
    }
}

fn collect_statement_assignment_names(statement: &Stmt, names: &mut Vec<String>) {
    match statement {
        Stmt::Assignment { target: Place::Binding { name, .. }, .. }
        | Stmt::CompoundAssignment { target: Place::Binding { name, .. }, .. } => {
            names.push(name.clone());
        }
        Stmt::If { then_branch, else_branch, .. } => {
            collect_assignment_names(then_branch, names);
            if let Some(IfBranch::Block(block)) = else_branch {
                collect_assignment_names(block, names);
            }
        }
        Stmt::Loop(block) | Stmt::Block(block) => collect_assignment_names(block, names),
        Stmt::ForRange { body, .. } | Stmt::ForArray { body, .. } => {
            collect_assignment_names(body, names)
        }
        Stmt::Expression { expression, .. }
        | Stmt::OwnerDecl { initializer: expression, .. }
        | Stmt::Assignment { value: expression, .. }
        | Stmt::CompoundAssignment { value: expression, .. } => {
            collect_expression_assignment_names(expression, names)
        }
        _ => {}
    }
}

pub(super) fn collect_expression_assignment_names(expression: &Expr, names: &mut Vec<String>) {
    match expression {
        Expr::Case { branches, .. } => {
            for branch in branches {
                match &branch.body {
                    CaseBody::Block(block) => collect_assignment_names(block, names),
                    CaseBody::Expression(expression) => {
                        collect_expression_assignment_names(expression, names)
                    }
                }
            }
        }
        Expr::If { then_branch, else_branch, .. } => {
            collect_assignment_names(then_branch, names);
            if let Some(IfBranch::Block(block)) = else_branch {
                collect_assignment_names(block, names);
            }
        }
        _ => {}
    }
}
