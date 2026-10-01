use std::collections::HashMap;

use crate::ast::{Block, CaseBody, Expr, IfBranch, Program, Stmt, TopLevelDecl};

/// Inlines validated constant values before native lowering.
///
/// Constants have no runtime storage or ABI identity. Keeping this rewrite in
/// codegen preparation makes that contract explicit while leaving semantic
/// validation in the semantic phase.
pub(super) fn inline_constants(program: &mut Program) {
    let constants = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Constant(constant) => {
                Some((constant.name.clone(), constant.initializer.clone()))
            }
            _ => None,
        })
        .collect::<HashMap<_, _>>();
    for declaration in &mut program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => inline_block(&mut verb.body, &constants),
            TopLevelDecl::Perform(perform) => {
                for method in &mut perform.methods {
                    inline_block(&mut method.body, &constants);
                }
            }
            _ => {}
        }
    }
}

fn inline_block(block: &mut Block, constants: &HashMap<String, Expr>) {
    for statement in &mut block.statements {
        match statement {
            Stmt::OwnerDecl { initializer, .. } | Stmt::Assignment { value: initializer, .. } => {
                inline_expression(initializer, constants)
            }
            Stmt::FieldAssignment { object, value, .. } => {
                inline_expression(object, constants);
                inline_expression(value, constants);
            }
            Stmt::IndexAssignment { target, index, value, .. } => {
                inline_expression(target, constants);
                inline_expression(index, constants);
                inline_expression(value, constants);
            }
            Stmt::CompoundAssignment { target, value, .. } => {
                match target {
                    crate::ast::CompoundAssignmentTarget::Field { object, .. } => {
                        inline_expression(object, constants)
                    }
                    crate::ast::CompoundAssignmentTarget::Index { target, index } => {
                        inline_expression(target, constants);
                        inline_expression(index, constants);
                    }
                    crate::ast::CompoundAssignmentTarget::Identifier(_) => {}
                }
                inline_expression(value, constants);
            }
            Stmt::Expression { expression, .. } => inline_expression(expression, constants),
            Stmt::Return { value: Some(expression), .. } => {
                inline_expression(expression, constants)
            }
            Stmt::Loop(nested) | Stmt::Block(nested) => inline_block(nested, constants),
            Stmt::Return { value: None, .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Drop { .. } => {}
        }
    }
}

fn inline_expression(expression: &mut Expr, constants: &HashMap<String, Expr>) {
    if let Expr::Identifier { name, .. } = expression
        && let Some(replacement) = constants.get(name)
    {
        *expression = replacement.clone();
        inline_expression(expression, constants);
        return;
    }
    match expression {
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. } => inline_expression(expression, constants),
        Expr::Binary { left, right, .. } => {
            inline_expression(left, constants);
            inline_expression(right, constants);
        }
        Expr::Call { arguments, .. } | Expr::MethodCall { arguments, .. } => {
            for argument in arguments {
                inline_expression(&mut argument.expression, constants);
            }
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                inline_expression(&mut field.value, constants);
            }
        }
        Expr::FieldAccess { object, .. } | Expr::Index { target: object, .. } => {
            inline_expression(object, constants);
            if let Expr::Index { index, .. } = expression {
                inline_expression(index, constants);
            }
        }
        Expr::Case { subject, branches, .. } => {
            inline_expression(subject, constants);
            for branch in branches {
                if let Some(guard) = &mut branch.guard {
                    inline_expression(guard, constants);
                }
                match &mut branch.body {
                    CaseBody::Expression(value) => inline_expression(value, constants),
                    CaseBody::Block(block) => inline_block(block, constants),
                }
            }
        }
        Expr::If { condition, then_branch, else_branch, .. } => {
            inline_conditional(condition, then_branch, else_branch.as_mut(), constants);
        }
        Expr::Integer { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. }
        | Expr::Identifier { .. } => {}
    }
}

fn inline_conditional(
    condition: &mut Expr,
    then_branch: &mut Block,
    else_branch: Option<&mut IfBranch>,
    constants: &HashMap<String, Expr>,
) {
    inline_expression(condition, constants);
    inline_block(then_branch, constants);
    if let Some(branch) = else_branch {
        match branch {
            IfBranch::Block(block) => inline_block(block, constants),
            IfBranch::ElseIf(expression) => inline_expression(expression, constants),
        }
    }
}
