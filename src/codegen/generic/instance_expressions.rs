use crate::ast::{CaseBody, Expr, IfBranch, Stmt, TypeName};

pub(super) fn collect_block_type_names(
    block: &crate::ast::Block,
    mut visit: impl FnMut(&TypeName),
) {
    collect_block_type_names_inner(block, &mut visit);
}

fn collect_block_type_names_inner(block: &crate::ast::Block, visit: &mut dyn FnMut(&TypeName)) {
    for statement in &block.statements {
        collect_statement_type_names(statement, visit);
    }
}

fn collect_statement_type_names(statement: &Stmt, visit: &mut dyn FnMut(&TypeName)) {
    match statement {
        Stmt::OwnerDecl { initializer, .. } | Stmt::Expression { expression: initializer, .. } => {
            collect_expression_type_names(initializer, visit)
        }
        Stmt::Assignment { value, .. } | Stmt::CompoundAssignment { value, .. } => {
            collect_expression_type_names(value, visit)
        }
        Stmt::If { condition, then_branch, else_branch, .. } => {
            collect_expression_type_names(condition, visit);
            collect_block_type_names_inner(then_branch, visit);
            if let Some(branch) = else_branch {
                collect_if_branch_type_names(branch, visit);
            }
        }
        Stmt::Return { value: Some(value), .. } => collect_expression_type_names(value, visit),
        Stmt::Loop(block) | Stmt::Block(block) => collect_block_type_names_inner(block, visit),
        Stmt::ForRange { start, end, body, .. } => {
            collect_expression_type_names(start, visit);
            collect_expression_type_names(end, visit);
            collect_block_type_names_inner(body, visit);
        }
        Stmt::ForArray { collection, body, .. } => {
            collect_expression_type_names(collection, visit);
            collect_block_type_names_inner(body, visit);
        }
        Stmt::Return { value: None, .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Drop { .. } => {}
    }
}

fn collect_if_branch_type_names(branch: &IfBranch, visit: &mut dyn FnMut(&TypeName)) {
    match branch {
        IfBranch::Block(block) => collect_block_type_names_inner(block, visit),
        IfBranch::ElseIf(expression) => collect_expression_type_names(expression, visit),
    }
}

fn collect_expression_type_names(expression: &Expr, visit: &mut dyn FnMut(&TypeName)) {
    match expression {
        Expr::Cast { expression, target, .. } => {
            visit(target);
            collect_expression_type_names(expression, visit);
        }
        Expr::BufferLiteral { length, .. }
        | Expr::Grouping { expression: length, .. }
        | Expr::Unary { expression: length, .. }
        | Expr::Borrow { expression: length, .. }
        | Expr::Try { expression: length, .. } => collect_expression_type_names(length, visit),
        Expr::Binary { left, right, .. } => {
            collect_expression_type_names(left, visit);
            collect_expression_type_names(right, visit);
        }
        Expr::Call { arguments, .. } => collect_arguments_type_names(arguments, visit),
        Expr::MethodCall { receiver, arguments, span, .. } => {
            collect_method_call_type_names(receiver, arguments, *span, visit)
        }
        Expr::StructLit { name, type_arguments, fields, span } => {
            visit(&TypeName {
                name: name.clone(),
                arguments: type_arguments.clone(),
                reference_role: None,
                span: *span,
            });
            for field in fields {
                collect_expression_type_names(&field.value, visit);
            }
        }
        Expr::FieldAccess { object, .. } => collect_expression_type_names(object, visit),
        Expr::Index { target, index, .. } => {
            collect_expression_type_names(target, visit);
            collect_expression_type_names(index, visit);
        }
        Expr::Case { subject, branches, .. } => collect_case_type_names(subject, branches, visit),
        Expr::If { condition, then_branch, else_branch, .. } => {
            collect_if_type_names(condition, then_branch, else_branch.as_ref(), visit)
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => {}
    }
}

fn collect_method_call_type_names(
    receiver: &Expr,
    arguments: &[crate::ast::Argument],
    span: crate::lexer::SourceSpan,
    visit: &mut dyn FnMut(&TypeName),
) {
    if let Expr::Identifier { name, .. } = receiver
        && let Some(type_name) = crate::semantic::parse_type_name_key(name, span)
    {
        visit(&type_name);
    }
    collect_expression_type_names(receiver, visit);
    collect_arguments_type_names(arguments, visit);
}

fn collect_case_type_names(
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    visit: &mut dyn FnMut(&TypeName),
) {
    collect_expression_type_names(subject, visit);
    for branch in branches {
        if let Some(guard) = &branch.guard {
            collect_expression_type_names(guard, visit);
        }
        match &branch.body {
            CaseBody::Expression(expression) => collect_expression_type_names(expression, visit),
            CaseBody::Block(block) => collect_block_type_names_inner(block, visit),
        }
    }
}

fn collect_if_type_names(
    condition: &Expr,
    then_branch: &crate::ast::Block,
    else_branch: Option<&IfBranch>,
    visit: &mut dyn FnMut(&TypeName),
) {
    collect_expression_type_names(condition, visit);
    collect_block_type_names_inner(then_branch, visit);
    if let Some(branch) = else_branch {
        collect_if_branch_type_names(branch, visit);
    }
}

fn collect_arguments_type_names(
    arguments: &[crate::ast::Argument],
    visit: &mut dyn FnMut(&TypeName),
) {
    for argument in arguments {
        collect_expression_type_names(&argument.expression, visit);
    }
}
