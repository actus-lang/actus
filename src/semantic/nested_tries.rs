//! A `?` nested inside an expression (bugs #128, #130), such as
//! `Pair { a: track(), b: load()? }`, `measure(value: parse()?) + 1`,
//! `if ready()? { .. }` or `left && check()?`, leaves the verb in the middle of
//! a statement. Its early return must release every value already evaluated in
//! that statement (ADR-0125), so the statement is rewritten before analysis
//! into the same steps written one per statement: each value-producing operand
//! evaluated before the `?` is bound to a generated `erg` local in evaluation
//! order, then the `?` itself is bound to one, and the statement reads those
//! locals. The order of evaluation does not change, the locals are ordinary
//! owners, and the exit list of the `?` releases the ones that are live there
//! like any other owner.
//!
//! Operands that are always evaluated are rewritten: the payload of an enum
//! constructor, the fields of a struct literal, the operands of an arithmetic
//! or comparison operator, the left operand of `&&` and `||`, the condition of
//! an `if`, the subject of a `case`, and the operand of a field read, an index,
//! a cast, a borrow or a unary operator.
//!
//! A `?` that runs on one path only, inside an arm of an `if` or a `case` or in
//! the right operand of `&&` or `||`, is not moved out of its path. `l && r`
//! with a `?` in `r` becomes `if l { r } else { false }` (`||` becomes
//! `if l { true } else { r }`), which keeps the short circuit, and the whole
//! conditional is bound to a generated local unless it is already the value of
//! the statement; its arms are then normalized as blocks of their own (see
//! `conditional_tries`), so the `?` is a statement of the arm.
//!
//! The arguments of a verb or a method are not rewritten: an argument must be
//! a place its role binds, so a value produced there, with or without a `?`,
//! keeps its rejection (`try_positions`).

use crate::ast::{BinaryOp, Block, Expr, IfBranch, Role, Stmt};
use crate::lexer::SourceSpan;

/// Rewrites the nested `?`s of `statement` and returns the statements that
/// must run before it. A conditional that is the whole value of the statement
/// keeps its arms when the statement lowers them as blocks: the value of an
/// `erg`, a `return` or an expression statement, and a `case` assigned to a
/// place. The condition of an `if` (with an `else if` that holds a `?` turned
/// into an `else` block first), the bounds of a `for`, its collection and the
/// value of a compound assignment are evaluated before anything else of the
/// statement, so every `?` in them is bound.
pub(super) fn hoist_statement(
    statement: &mut Stmt,
    fresh_name: &mut dyn FnMut() -> String,
) -> Vec<Stmt> {
    match statement {
        Stmt::OwnerDecl { initializer: value, .. }
        | Stmt::Return { value: Some(value), .. }
        | Stmt::Expression { expression: value, .. } => hoist(value, true, fresh_name),
        Stmt::Assignment { value, .. } => {
            let keep = matches!(value, Expr::Case { .. });
            hoist(value, keep, fresh_name)
        }
        Stmt::CompoundAssignment { value, .. } => hoist_all(vec![value], fresh_name),
        Stmt::If { condition, else_branch, .. } => {
            super::conditional_tries::split_else_if(else_branch);
            hoist_all(vec![condition], fresh_name)
        }
        Stmt::ForRange { start, end, .. } => hoist_all(vec![start, end.as_mut()], fresh_name),
        Stmt::ForArray { collection, .. } => hoist_all(vec![collection], fresh_name),
        _ => Vec::new(),
    }
}

/// Rewrites every nested `?` of `expression`, the value of a statement, and
/// returns the statements that must run before it. A `?` that is the whole
/// value (`erg x = load()?;`) is already a statement of its own and is kept;
/// only the `?`s inside its operand are rewritten. A conditional that is the
/// whole value is kept as well when `keep_root_conditional` holds, because
/// the statement lowers its arms as blocks; only its condition or subject is
/// rewritten then.
fn hoist(
    expression: &mut Expr,
    keep_root_conditional: bool,
    fresh_name: &mut dyn FnMut() -> String,
) -> Vec<Stmt> {
    rewrite_logical_tries(expression);
    let mut prefix = Vec::new();
    match expression {
        Expr::Try { expression: operand, .. } => hoist_inside(operand, fresh_name, &mut prefix),
        Expr::If { .. } | Expr::Case { .. } if keep_root_conditional => {
            hoist_in_order(operands_mut(expression), fresh_name, &mut prefix)
        }
        _ => hoist_inside(expression, fresh_name, &mut prefix),
    }
    prefix
}

/// Rewrites every `?` of `expressions`, which a statement evaluates in this
/// order before anything else (an `if` condition, the bounds of a `for`, the
/// value of a compound assignment), including a `?` that is a whole
/// expression, and returns the statements that must run before the statement.
fn hoist_all(mut expressions: Vec<&mut Expr>, fresh_name: &mut dyn FnMut() -> String) -> Vec<Stmt> {
    for expression in &mut expressions {
        rewrite_logical_tries(expression);
    }
    let mut prefix = Vec::new();
    hoist_in_order(expressions, fresh_name, &mut prefix);
    prefix
}

/// Rewrites the nested `?`s of `expression` in evaluation order, binding
/// each one, and the operands evaluated before it, to a generated local. A
/// conditional with a `?` on one of its paths is bound as a whole, after its
/// condition or subject.
fn hoist_inside(
    expression: &mut Expr,
    fresh_name: &mut dyn FnMut() -> String,
    prefix: &mut Vec<Stmt>,
) {
    if !contains_try(expression) {
        return;
    }
    let bound_whole = matches!(expression, Expr::Try { .. }) || has_conditional_try(expression);
    match expression {
        Expr::Try { expression: operand, .. } => hoist_inside(operand, fresh_name, prefix),
        _ => hoist_in_order(operands_mut(expression), fresh_name, prefix),
    }
    if bound_whole {
        let span = expression_span(expression);
        let value = std::mem::replace(expression, Expr::Identifier { name: String::new(), span });
        *expression = bind(value, span, fresh_name, prefix);
    }
}

/// Rewrites the operands `operands`, evaluated in this order: the operands
/// before the last one that holds a `?` are bound when they produce a value,
/// so they still run first, and the `?`s of that last one are rewritten.
fn hoist_in_order(
    mut operands: Vec<&mut Expr>,
    fresh_name: &mut dyn FnMut() -> String,
    prefix: &mut Vec<Stmt>,
) {
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
pub(super) fn bind(
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

/// Rewrites `l && r` and `l || r` whose right operand holds a `?` into the
/// `if` that evaluates `r` only on its own path, here and in every operand
/// that is always evaluated.
fn rewrite_logical_tries(expression: &mut Expr) {
    if let Expr::Binary { left, operator, right, span } = expression
        && matches!(operator, BinaryOp::LogicalAnd | BinaryOp::LogicalOr)
        && contains_any_try(right)
    {
        let span = *span;
        let arm = |value: Expr| Block {
            statements: vec![Stmt::Expression { expression: value, span }],
            span,
        };
        let constant = |value: bool| Expr::BoolLiteral { value, span };
        let right = std::mem::replace(right.as_mut(), constant(false));
        let (then_value, else_value) = if *operator == BinaryOp::LogicalAnd {
            (right, constant(false))
        } else {
            (constant(true), right)
        };
        let condition = std::mem::replace(left.as_mut(), constant(false));
        *expression = Expr::If {
            condition: Box::new(condition),
            then_branch: arm(then_value),
            else_branch: Some(IfBranch::Block(arm(else_value))),
            span,
        };
    }
    for operand in operands_mut(expression) {
        rewrite_logical_tries(operand);
    }
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
        // A constructor of a type application (`Result[T, E].Ok(...)`) is
        // parsed as a method of the type's name.
        Expr::MethodCall { receiver, arguments, .. } if names_a_type(receiver) => {
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
        Expr::If { condition: expression, .. }
        | Expr::Case { subject: expression, .. }
        | Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::FieldAccess { object: expression, .. } => vec![expression.as_mut()],
        _ => Vec::new(),
    }
}

/// Whether `receiver` is a type name, such as `Result[Int, Fail]`, so that
/// a method called on it is an enum constructor.
pub(super) fn names_a_type(receiver: &Expr) -> bool {
    matches!(receiver, Expr::Identifier { name, .. }
        if name.starts_with(|first: char| first.is_ascii_uppercase()))
}

/// Whether `expression` holds a `?` that this rewrite binds: in an operand
/// that is always evaluated, or on one path of a conditional.
fn contains_try(expression: &Expr) -> bool {
    if matches!(expression, Expr::Try { .. }) || has_conditional_try(expression) {
        return true;
    }
    let mut copy = expression.clone();
    operands_mut(&mut copy).into_iter().any(|operand| contains_try(operand))
}

/// Whether `expression` is an `if`, a `case`, `&&` or `||` with a `?` on a
/// path that runs only when it is selected.
pub(super) fn has_conditional_try(expression: &Expr) -> bool {
    match expression {
        Expr::If { then_branch, else_branch, .. } => {
            super::conditional_tries::block_has_try(then_branch)
                || else_branch.as_ref().is_some_and(super::conditional_tries::branch_has_try)
        }
        Expr::Case { branches, .. } => {
            branches.iter().any(super::conditional_tries::case_branch_has_try)
        }
        Expr::Binary { operator: BinaryOp::LogicalAnd | BinaryOp::LogicalOr, right, .. } => {
            contains_any_try(right)
        }
        _ => false,
    }
}

/// Whether a `?` appears anywhere in `expression`, on any path.
pub(super) fn contains_any_try(expression: &Expr) -> bool {
    super::conditional_tries::expression_has_try(expression)
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

pub(super) fn expression_span(expression: &Expr) -> SourceSpan {
    crate::semantic::calls::shapes::expression_span(expression)
}
