//! A `?` nested inside an expression (bug #128), such as
//! `Pair { a: track(), b: load()? }` or `measure(value: parse()?)`, leaves
//! the verb in the middle of a statement. Its early return must release every
//! value already evaluated in that statement (ADR-0125), so the statement is
//! rewritten before analysis into the same steps written one per statement:
//! each value-producing operand evaluated before the `?` is bound to a
//! generated `erg` local in evaluation order, then the `?` itself is bound to
//! one, and the statement reads those locals. The order of evaluation does
//! not change, the locals are ordinary owners, and the exit list of the `?`
//! releases the ones that are live there like any other owner.
//!
//! Only operands that are always evaluated are rewritten: the payload of an
//! enum constructor, the fields of a struct literal, the operands of an
//! arithmetic or comparison operator and the operand of a field read, an
//! index, a cast or a unary operator. The arguments of a verb or a method are
//! not: an argument must be a place its role binds, so a value produced there,
//! with or without a `?`, keeps its rejection. A `?` inside an `if`, a `case`, or the
//! right operand of `&&` or `||` is evaluated on one path only and stays as
//! it is.

use crate::ast::{BinaryOp, Expr, Role, Stmt};
use crate::lexer::SourceSpan;

/// Rewrites every nested `?` of `expression`, the value of a statement, and
/// returns the statements that must run before it. A `?` that is the whole
/// value (`erg x = load()?;`) is already a statement of its own and is kept;
/// only the `?`s inside its operand are rewritten.
pub(super) fn hoist(expression: &mut Expr, fresh_name: &mut dyn FnMut() -> String) -> Vec<Stmt> {
    let mut prefix = Vec::new();
    match expression {
        Expr::Try { expression: operand, .. } => hoist_inside(operand, fresh_name, &mut prefix),
        _ => hoist_inside(expression, fresh_name, &mut prefix),
    }
    prefix
}

/// Rewrites the nested `?`s of `expression` in evaluation order, binding
/// each one, and the operands evaluated before it, to a generated local.
fn hoist_inside(
    expression: &mut Expr,
    fresh_name: &mut dyn FnMut() -> String,
    prefix: &mut Vec<Stmt>,
) {
    if !contains_try(expression) {
        return;
    }
    if let Expr::Try { expression: operand, span } = expression {
        hoist_inside(operand, fresh_name, prefix);
        let span = *span;
        let value = std::mem::replace(expression, Expr::Identifier { name: String::new(), span });
        *expression = bind(value, span, fresh_name, prefix);
        return;
    }
    let mut operands = operands_mut(expression);
    let Some(last) = operands.iter().rposition(|operand| contains_try(operand)) else { return };
    for operand in operands.iter_mut().take(last) {
        if produces_value(operand) {
            let span = expression_span(operand);
            let value = std::mem::replace(*operand, Expr::Identifier { name: String::new(), span });
            **operand = bind(value, span, fresh_name, prefix);
        }
    }
    hoist_inside(operands[last], fresh_name, prefix);
}

/// Binds `value` to a new generated `erg` local and returns the read of it.
fn bind(
    value: Expr,
    span: SourceSpan,
    fresh_name: &mut dyn FnMut() -> String,
    prefix: &mut Vec<Stmt>,
) -> Expr {
    let name = fresh_name();
    prefix.push(Stmt::OwnerDecl {
        role: Role::Erg,
        name: name.clone(),
        ty: None,
        resolved_type: None,
        initializer: value,
        span,
    });
    Expr::Identifier { name, span }
}

/// The operands of `expression` that are always evaluated, in evaluation
/// order.
fn operands_mut(expression: &mut Expr) -> Vec<&mut Expr> {
    match expression {
        // A constructor (`Msg.Data(...)`, `Option[T].Some(...)`, named in
        // PascalCase) takes its payload by value. The arguments of a verb or
        // a method are places that their roles bind, and a value produced in
        // place of one is rejected with or without a `?`, so a `?` there is
        // left for the analyzer to judge as it is.
        Expr::Call { callee, arguments, .. }
            if callee.starts_with(|first: char| first.is_ascii_uppercase()) =>
        {
            arguments.iter_mut().map(|argument| &mut argument.expression).collect()
        }
        Expr::StructLit { fields, .. } => fields.iter_mut().map(|field| &mut field.value).collect(),
        Expr::Binary { left, operator, right, .. } => {
            if matches!(operator, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                vec![left.as_mut()]
            } else {
                vec![left.as_mut(), right.as_mut()]
            }
        }
        Expr::Index { target, index, .. } => vec![target.as_mut(), index.as_mut()],
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::FieldAccess { object: expression, .. } => vec![expression.as_mut()],
        _ => Vec::new(),
    }
}

/// Whether `expression` holds a `?` in an operand that is always evaluated.
fn contains_try(expression: &Expr) -> bool {
    if matches!(expression, Expr::Try { .. }) {
        return true;
    }
    let mut copy = expression.clone();
    operands_mut(&mut copy).into_iter().any(|operand| contains_try(operand))
}

/// Whether evaluating `expression` can do anything but read: a literal, a
/// binding, a field or element of one, and operators over those are read
/// where they stand; anything else is bound before the `?`.
fn produces_value(expression: &Expr) -> bool {
    match expression {
        Expr::Integer { .. }
        | Expr::FloatLiteral { .. }
        | Expr::BoolLiteral { .. }
        | Expr::StringLiteral { .. }
        | Expr::Identifier { .. } => false,
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::FieldAccess { object: expression, .. } => produces_value(expression),
        Expr::Binary { left, right, .. } => produces_value(left) || produces_value(right),
        Expr::Index { target, index, .. } => produces_value(target) || produces_value(index),
        _ => true,
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    crate::semantic::calls::shapes::expression_span(expression)
}
