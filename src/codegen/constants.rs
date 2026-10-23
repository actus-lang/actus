use std::collections::HashMap;

use crate::ast::{
    BinaryOp, Block, CaseBody, Expr, IfBranch, Place, Program, Stmt, TopLevelDecl, TypeName,
    UnaryOp,
};

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
            TopLevelDecl::Pack(pack) => {
                for field in &mut pack.fields {
                    if let Some(name) = field.offset_name.as_deref()
                        && let Some(value) =
                            evaluate_constant_name(name, &constants, &mut Vec::new())
                        && let Ok(offset) = u16::try_from(value)
                    {
                        field.offset = offset;
                    }
                }
            }
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

fn evaluate_constant_name(
    name: &str,
    constants: &HashMap<String, Expr>,
    path: &mut Vec<String>,
) -> Option<i128> {
    if path.iter().any(|entry| entry == name) {
        return None;
    }
    let expression = constants.get(name)?;
    path.push(name.to_owned());
    let value = evaluate_constant_expression(expression, constants, path);
    path.pop();
    value
}

fn evaluate_constant_expression(
    expression: &Expr,
    constants: &HashMap<String, Expr>,
    path: &mut Vec<String>,
) -> Option<i128> {
    match expression {
        Expr::Integer { value, .. } => value.parse().ok(),
        Expr::Identifier { name, .. } => evaluate_constant_name(name, constants, path),
        Expr::Grouping { expression, .. } => {
            evaluate_constant_expression(expression, constants, path)
        }
        Expr::Cast { expression, .. } => evaluate_constant_expression(expression, constants, path),
        Expr::Unary { operator, expression, .. } => {
            let value = evaluate_constant_expression(expression, constants, path)?;
            match operator {
                UnaryOp::Negate => value.checked_neg(),
                UnaryOp::BitwiseNot => Some(!value),
                UnaryOp::LogicalNot => None,
            }
        }
        Expr::Binary { left, operator, right, .. } => {
            let left = evaluate_constant_expression(left, constants, path)?;
            let right = evaluate_constant_expression(right, constants, path)?;
            match operator {
                BinaryOp::Add => left.checked_add(right),
                BinaryOp::Subtract => left.checked_sub(right),
                BinaryOp::Multiply => left.checked_mul(right),
                BinaryOp::Divide => left.checked_div(right),
                BinaryOp::Remainder => left.checked_rem(right),
                BinaryOp::BitwiseAnd => Some(left & right),
                BinaryOp::BitwiseOr => Some(left | right),
                BinaryOp::BitwiseXor => Some(left ^ right),
                BinaryOp::ShiftLeft => left.checked_shl(u32::try_from(right).ok()?),
                BinaryOp::ShiftRight => left.checked_shr(u32::try_from(right).ok()?),
                _ => None,
            }
        }
        _ => None,
    }
}

fn inline_block(block: &mut Block, constants: &HashMap<String, Expr>) {
    for statement in &mut block.statements {
        match statement {
            Stmt::OwnerDecl { initializer, .. } => inline_expression(initializer, constants),
            Stmt::Assignment { target, value, .. }
            | Stmt::CompoundAssignment { target, value, .. } => {
                inline_place(target, constants);
                inline_expression(value, constants);
            }
            Stmt::Expression { expression, .. } => inline_expression(expression, constants),
            Stmt::If { condition, then_branch, else_branch, .. } => {
                inline_expression(condition, constants);
                inline_block(then_branch, constants);
                if let Some(branch) = else_branch {
                    match branch {
                        IfBranch::Block(block) => inline_block(block, constants),
                        IfBranch::ElseIf(expression) => inline_expression(expression, constants),
                    }
                }
            }
            Stmt::Return { value: Some(expression), .. } => {
                inline_expression(expression, constants)
            }
            Stmt::Loop(nested) | Stmt::Block(nested) => inline_block(nested, constants),
            Stmt::ForRange { start, end, body, .. } => {
                inline_expression(start, constants);
                inline_expression(end, constants);
                inline_block(body, constants);
            }
            Stmt::ForArray { collection, body, .. } => {
                inline_expression(collection, constants);
                inline_block(body, constants);
            }
            Stmt::Return { value: None, .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Drop { .. } => {}
        }
    }
}

fn inline_place(place: &mut Place, constants: &HashMap<String, Expr>) {
    match place {
        Place::Binding { .. } => {}
        Place::Field { object, .. } => inline_place(object, constants),
        Place::Index { target, index, .. } => {
            inline_place(target, constants);
            inline_expression(index, constants);
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
    inline_expression_node(expression, constants);
}

fn inline_expression_node(expression: &mut Expr, constants: &HashMap<String, Expr>) {
    match expression {
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            inline_expression(expression, constants)
        }
        Expr::Cast { expression, target, .. } => {
            inline_type_name(target, constants);
            inline_expression(expression, constants);
        }
        Expr::Borrow { expression, .. } | Expr::Try { expression, .. } => {
            inline_expression(expression, constants)
        }
        Expr::Binary { left, right, .. } => {
            inline_expression(left, constants);
            inline_expression(right, constants);
        }
        Expr::Call { callee, arguments, span } => inline_call(callee, arguments, *span, constants),
        Expr::MethodCall { receiver, arguments, .. } => {
            inline_method_call(receiver, arguments, constants);
        }
        Expr::StructLit { type_arguments, fields, .. } => {
            inline_struct_literal(type_arguments, fields, constants);
        }
        Expr::FieldAccess { object, .. } => inline_expression(object, constants),
        Expr::Index { target: object, index, .. } => {
            inline_expression(object, constants);
            inline_expression(index, constants);
        }
        Expr::Case { subject, branches, .. } => inline_case(subject, branches, constants),
        Expr::If { condition, then_branch, else_branch, .. } => {
            inline_conditional(condition, then_branch, else_branch.as_mut(), constants);
        }
        Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. }
        | Expr::Identifier { .. } => {}
    }
}

fn inline_call(
    callee: &mut String,
    arguments: &mut [crate::ast::Argument],
    span: crate::lexer::SourceSpan,
    constants: &HashMap<String, Expr>,
) {
    inline_array_callee(callee, span, constants);
    for argument in arguments {
        inline_expression(&mut argument.expression, constants);
    }
}

fn inline_method_call(
    receiver: &mut Box<Expr>,
    arguments: &mut [crate::ast::Argument],
    constants: &HashMap<String, Expr>,
) {
    inline_expression(receiver, constants);
    for argument in arguments {
        inline_expression(&mut argument.expression, constants);
    }
}

fn inline_struct_literal(
    type_arguments: &mut [TypeName],
    fields: &mut [crate::ast::StructFieldInit],
    constants: &HashMap<String, Expr>,
) {
    for type_argument in type_arguments {
        inline_type_name(type_argument, constants);
    }
    for field in fields {
        inline_expression(&mut field.value, constants);
    }
}

fn inline_case(
    subject: &mut Box<Expr>,
    branches: &mut [crate::ast::CaseBranch],
    constants: &HashMap<String, Expr>,
) {
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

fn inline_array_callee(
    callee: &mut String,
    span: crate::lexer::SourceSpan,
    constants: &HashMap<String, Expr>,
) {
    if !callee.starts_with("Array[") {
        return;
    }
    let Some(mut type_name) = crate::semantic::parse_type_name_key(callee, span) else {
        return;
    };
    inline_type_name(&mut type_name, constants);
    *callee = type_name.canonical_key();
}

fn inline_type_name(type_name: &mut TypeName, constants: &HashMap<String, Expr>) {
    for argument in &mut type_name.arguments {
        inline_type_name(argument, constants);
    }
    if type_name.name != "Array" || type_name.arguments.len() != 2 {
        return;
    }
    let capacity = &mut type_name.arguments[1];
    if let Some(value) = evaluate_constant_name(&capacity.name, constants, &mut Vec::new()) {
        capacity.name = value.to_string();
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
