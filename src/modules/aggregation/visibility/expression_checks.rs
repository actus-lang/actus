use std::collections::HashSet;

use crate::ast::{Block, CaseBody, Expr, IfBranch, Stmt};

use super::super::types::ModuleError;
use super::super::unit::ModuleUnit;

struct VisibilityContext<'a> {
    private_verbs: &'a HashSet<String>,
    private_types: &'a HashSet<String>,
    module_path: &'a str,
    unit: &'a ModuleUnit,
}

pub(super) fn check_block(
    block: &Block,
    private_verbs: &HashSet<String>,
    private_types: &HashSet<String>,
    module_path: &str,
    unit: &ModuleUnit,
) -> Result<(), ModuleError> {
    let context = VisibilityContext { private_verbs, private_types, module_path, unit };
    check_block_with_context(block, &context)
}

fn check_block_with_context(
    block: &Block,
    context: &VisibilityContext<'_>,
) -> Result<(), ModuleError> {
    for statement in &block.statements {
        match statement {
            Stmt::OwnerDecl { initializer, ty, span, .. } => {
                if let Some(type_name) = ty {
                    super::check_type_name(
                        type_name,
                        context.private_types,
                        context.module_path,
                        context.unit,
                        *span,
                    )?;
                }
                check_expr_with_context(initializer, context)?;
            }
            Stmt::Assignment { value, .. }
            | Stmt::Expression { expression: value, .. }
            | Stmt::Return { value: Some(value), .. } => check_expr_with_context(value, context)?,
            Stmt::FieldAssignment { object, value, .. } => {
                check_expr_with_context(object, context)?;
                check_expr_with_context(value, context)?;
            }
            Stmt::IndexAssignment { target, index, value, .. } => {
                check_expr_with_context(target, context)?;
                check_expr_with_context(index, context)?;
                check_expr_with_context(value, context)?;
            }
            Stmt::CompoundAssignment { target, value, .. } => {
                match target {
                    crate::ast::CompoundAssignmentTarget::Identifier(_) => {}
                    crate::ast::CompoundAssignmentTarget::Field { object, .. } => {
                        check_expr_with_context(object, context)?;
                    }
                    crate::ast::CompoundAssignmentTarget::Index { target, index } => {
                        check_expr_with_context(target, context)?;
                        check_expr_with_context(index, context)?;
                    }
                }
                check_expr_with_context(value, context)?;
            }
            Stmt::Return { value: None, .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Loop(nested) | Stmt::Block(nested) => {
                check_block_with_context(nested, context)?;
            }
            Stmt::Drop { .. } => {}
        }
    }
    Ok(())
}

fn check_expr_with_context(
    expression: &Expr,
    context: &VisibilityContext<'_>,
) -> Result<(), ModuleError> {
    match expression {
        Expr::Call { .. } => check_call(expression, context)?,
        Expr::StructLit { .. } => check_struct_literal(expression, context)?,
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. } => check_expr_with_context(expression, context)?,
        Expr::Cast { .. } => check_cast(expression, context)?,
        Expr::Binary { .. } => check_binary(expression, context)?,
        Expr::MethodCall { .. } => check_method_call(expression, context)?,
        Expr::FieldAccess { .. } | Expr::Index { .. } => check_access(expression, context)?,
        Expr::Case { .. } => check_case(expression, context)?,
        Expr::If { .. } => check_if_expression(expression, context)?,
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => {}
    }
    Ok(())
}

fn check_call(expression: &Expr, context: &VisibilityContext<'_>) -> Result<(), ModuleError> {
    let Expr::Call { callee, span, arguments } = expression else { unreachable!() };
    super::reject_private(callee, context.private_verbs, context.module_path, context.unit, *span)?;
    for argument in arguments {
        check_expr_with_context(&argument.expression, context)?;
    }
    Ok(())
}

fn check_struct_literal(
    expression: &Expr,
    context: &VisibilityContext<'_>,
) -> Result<(), ModuleError> {
    let Expr::StructLit { name, span, fields, .. } = expression else { unreachable!() };
    super::reject_private(name, context.private_types, context.module_path, context.unit, *span)?;
    for field in fields {
        check_expr_with_context(&field.value, context)?;
    }
    Ok(())
}

fn check_cast(expression: &Expr, context: &VisibilityContext<'_>) -> Result<(), ModuleError> {
    let Expr::Cast { expression, target, span } = expression else { unreachable!() };
    super::check_type_name(
        &target.name,
        context.private_types,
        context.module_path,
        context.unit,
        *span,
    )?;
    check_expr_with_context(expression, context)
}

fn check_binary(expression: &Expr, context: &VisibilityContext<'_>) -> Result<(), ModuleError> {
    let Expr::Binary { left, right, .. } = expression else { unreachable!() };
    check_expr_with_context(left, context)?;
    check_expr_with_context(right, context)
}

fn check_method_call(
    expression: &Expr,
    context: &VisibilityContext<'_>,
) -> Result<(), ModuleError> {
    let Expr::MethodCall { receiver, arguments, .. } = expression else { unreachable!() };
    check_expr_with_context(receiver, context)?;
    for argument in arguments {
        check_expr_with_context(&argument.expression, context)?;
    }
    Ok(())
}

fn check_access(expression: &Expr, context: &VisibilityContext<'_>) -> Result<(), ModuleError> {
    let object = match expression {
        Expr::FieldAccess { object, .. } | Expr::Index { target: object, .. } => object,
        _ => unreachable!(),
    };
    check_expr_with_context(object, context)?;
    if let Expr::Index { index, .. } = expression {
        check_expr_with_context(index, context)?;
    }
    Ok(())
}

fn check_case(expression: &Expr, context: &VisibilityContext<'_>) -> Result<(), ModuleError> {
    let Expr::Case { subject, branches, .. } = expression else { unreachable!() };
    check_expr_with_context(subject, context)?;
    check_case_branches(branches, context)
}

fn check_case_branches(
    branches: &[crate::ast::CaseBranch],
    context: &VisibilityContext<'_>,
) -> Result<(), ModuleError> {
    for branch in branches {
        if let Some(guard) = &branch.guard {
            check_expr_with_context(guard, context)?;
        }
        match &branch.body {
            CaseBody::Expression(value) => check_expr_with_context(value, context)?,
            CaseBody::Block(block) => check_block_with_context(block, context)?,
        }
    }
    Ok(())
}

fn check_if_expression(
    expression: &Expr,
    context: &VisibilityContext<'_>,
) -> Result<(), ModuleError> {
    let Expr::If { condition, then_branch, else_branch, .. } = expression else {
        unreachable!("conditional visibility check received a non-conditional expression")
    };
    check_expr_with_context(condition, context)?;
    check_block_with_context(then_branch, context)?;
    if let Some(else_branch) = else_branch {
        match else_branch {
            IfBranch::Block(block) => check_block_with_context(block, context)?,
            IfBranch::ElseIf(expression) => check_if_expression(expression, context)?,
        }
    }
    Ok(())
}
