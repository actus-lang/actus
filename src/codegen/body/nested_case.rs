use crate::ast::{Block, Expr, Role, Stmt};

use crate::lexer::SourceSpan;

/// Hoist value-producing cases and conditionals nested inside scalar expressions.
pub(super) fn normalize_block(block: &Block) -> Block {
    let mut statements = Vec::new();
    for statement in &block.statements {
        normalize_statement(statement, &mut statements);
    }
    Block { statements, span: block.span }
}

fn normalize_statement(statement: &Stmt, output: &mut Vec<Stmt>) {
    match statement {
        Stmt::Return { value: Some(expression), span } => {
            let mut owners = Vec::new();
            let expression = extract_nested_cases(expression, &mut owners, true);
            output.extend(owners);
            output.push(Stmt::Return { value: Some(expression), span: *span });
        }
        Stmt::OwnerDecl { role, name, ty, initializer, span } => {
            let mut owners = Vec::new();
            let initializer = extract_nested_cases(initializer, &mut owners, true);
            output.extend(owners);
            output.push(Stmt::OwnerDecl {
                role: role.clone(),
                name: name.clone(),
                ty: ty.clone(),
                initializer,
                span: *span,
            });
        }
        Stmt::Assignment { target, value, span } => {
            let mut owners = Vec::new();
            let value = extract_nested_cases(value, &mut owners, true);
            output.extend(owners);
            output.push(Stmt::Assignment { target: target.clone(), value, span: *span });
        }
        Stmt::CompoundAssignment { target, operator, value, span } => {
            let mut owners = Vec::new();
            let value = extract_nested_cases(value, &mut owners, true);
            output.extend(owners);
            output.push(Stmt::CompoundAssignment {
                target: target.clone(),
                operator: *operator,
                value,
                span: *span,
            });
        }
        Stmt::Expression { expression, span } => {
            let mut owners = Vec::new();
            let expression = extract_nested_cases(expression, &mut owners, true);
            output.extend(owners);
            output.push(Stmt::Expression { expression, span: *span });
        }
        Stmt::If { condition, then_branch, else_branch, span } => {
            let mut owners = Vec::new();
            let condition = extract_nested_cases(condition, &mut owners, true);
            output.extend(owners);
            output.push(Stmt::If {
                condition,
                then_branch: normalize_block(then_branch),
                else_branch: else_branch.as_ref().map(normalize_if_branch),
                span: *span,
            });
        }
        _ => output.push(statement.clone()),
    }
}

fn normalize_if_branch(branch: &crate::ast::IfBranch) -> crate::ast::IfBranch {
    match branch {
        crate::ast::IfBranch::Block(block) => crate::ast::IfBranch::Block(normalize_block(block)),
        crate::ast::IfBranch::ElseIf(expression) => {
            crate::ast::IfBranch::ElseIf(Box::new(normalize_expression(expression)))
        }
    }
}

fn normalize_expression(expression: &Expr) -> Expr {
    match expression {
        Expr::If { condition, then_branch, else_branch, span } => Expr::If {
            condition: Box::new(normalize_expression(condition)),
            then_branch: normalize_block(then_branch),
            else_branch: else_branch.as_ref().map(normalize_if_branch),
            span: *span,
        },
        _ => expression.clone(),
    }
}

fn extract_nested_cases(expression: &Expr, owners: &mut Vec<Stmt>, root: bool) -> Expr {
    if !root && matches!(expression, Expr::Case { .. } | Expr::If { .. }) {
        let span = expression_span(expression);
        let prefix =
            if matches!(expression, Expr::Case { .. }) { "__nested_case" } else { "__nested_if" };
        let name = format!("{prefix}_{}_{}", span.start, span.end);
        owners.push(Stmt::OwnerDecl {
            role: Role::Erg,
            name: name.clone(),
            ty: None,
            initializer: expression.clone(),
            span,
        });
        return Expr::Identifier { name, span };
    }
    match expression {
        Expr::Binary { left, operator, right, span } => Expr::Binary {
            left: Box::new(extract_nested_cases(left, owners, false)),
            operator: *operator,
            right: Box::new(extract_nested_cases(right, owners, false)),
            span: *span,
        },
        Expr::Grouping { expression, span } => Expr::Grouping {
            expression: Box::new(extract_nested_cases(expression, owners, false)),
            span: *span,
        },
        _ => expression.clone(),
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => *span,
    }
}
