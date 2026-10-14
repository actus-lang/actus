use crate::ast::{Block, Expr, IfBranch, Stmt};

pub(super) fn block_has_nested_try(block: &Block) -> bool {
    block.statements.iter().any(|statement| match statement {
        Stmt::Expression { expression, .. }
        | Stmt::OwnerDecl { initializer: expression, .. }
        | Stmt::Assignment { value: expression, .. } => expression_has_nested_try(expression),
        Stmt::If { then_branch, else_branch, .. } => {
            block_has_nested_try(then_branch)
                || else_branch.as_ref().is_some_and(|branch| match branch {
                    IfBranch::Block(block) => block_has_nested_try(block),
                    IfBranch::ElseIf(expression) => expression_has_nested_try(expression),
                })
        }
        Stmt::Loop(block)
        | Stmt::ForRange { body: block, .. }
        | Stmt::ForArray { body: block, .. }
        | Stmt::Block(block) => block_has_nested_try(block),
        Stmt::Return { value, .. } => value.as_ref().is_some_and(expression_has_nested_try),
        _ => false,
    })
}

fn expression_has_nested_try(expression: &Expr) -> bool {
    match expression {
        Expr::Try { .. } => true,
        Expr::If { then_branch, else_branch, .. } => {
            block_has_nested_try(then_branch)
                || else_branch.as_ref().is_some_and(|branch| match branch {
                    IfBranch::Block(block) => block_has_nested_try(block),
                    IfBranch::ElseIf(expression) => expression_has_nested_try(expression),
                })
        }
        _ => false,
    }
}

pub(super) fn range_body_has_exit(block: &Block) -> bool {
    block.statements.iter().any(|statement| match statement {
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::If { then_branch, else_branch, .. } => {
            range_body_has_exit(then_branch)
                || else_branch.as_ref().is_some_and(|branch| match branch {
                    IfBranch::Block(block) => range_body_has_exit(block),
                    IfBranch::ElseIf(expression) => expression_has_nested_exit(expression),
                })
        }
        Stmt::Loop(block)
        | Stmt::Block(block)
        | Stmt::ForRange { body: block, .. }
        | Stmt::ForArray { body: block, .. } => range_body_has_exit(block),
        _ => false,
    })
}

fn expression_has_nested_exit(expression: &Expr) -> bool {
    match expression {
        Expr::If { then_branch, else_branch, .. } => {
            range_body_has_exit(then_branch)
                || else_branch.as_ref().is_some_and(|branch| match branch {
                    IfBranch::Block(block) => range_body_has_exit(block),
                    IfBranch::ElseIf(expression) => expression_has_nested_exit(expression),
                })
        }
        _ => false,
    }
}
