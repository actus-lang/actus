// Shared helpers for bounded loop lowering.

use super::super::BackendValueType;
use super::super::body_contract::unsupported;
use super::BackendContractError;
use crate::ast::{Block, ForBinding, IfBranch, Place, Stmt};
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
        _ => {}
    }
}
