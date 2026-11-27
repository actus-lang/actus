//! The positions a `?` may stand in (bug #130). After the normalization of
//! nested `?`s (`nested_tries`, `conditional_tries`) every `?` the lowering
//! supports is the whole value of a statement: an `erg` initializer, an
//! assignment, a `return` or an expression statement, possibly inside the arm
//! of a conditional. Those get their early return with the cleanup the
//! analyzer planned for them (ADR-0125). A `?` left anywhere else, in an
//! argument of a verb, a method or an intrinsic, in a `case` guard or in the
//! index of an assigned element, would not return early, so the program is
//! rejected with `E1016` instead of being lowered wrongly. The fix is to bind
//! the value first, `erg value = load()?;`, and use the binding.
//!
//! The arms of a conditional whose value an owner receives (`erg x = if ..`,
//! `x = case ..`) are lowered statement by statement; a `?` statement splits
//! them, but a conditional, a loop or an `if` statement nested in such an arm
//! is not lowered as blocks there, so a `?` inside one is rejected as well.

use crate::ast::{Block, CaseBody, CaseBranch, Expr, IfBranch, Place, Program, Stmt, TopLevelDecl};
use crate::lexer::SourceSpan;

/// The first `?` of the normalized `program` that is not the whole value of a
/// statement, with a description of the place it stands in.
pub(super) fn misplaced_try(program: &Program) -> Option<(SourceSpan, String)> {
    program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Verb(verb) => block_try(&verb.body, false),
        TopLevelDecl::Perform(perform) => {
            perform.methods.iter().find_map(|method| block_try(&method.body, false))
        }
        _ => None,
    })
}

/// The first misplaced `?` of `block`; `linear` holds for the arm of a
/// conditional that an owner receives.
fn block_try(block: &Block, linear: bool) -> Option<(SourceSpan, String)> {
    block.statements.iter().find_map(|statement| statement_try(statement, linear))
}

/// A misplaced `?` of `statement`; a `?` that is the whole value of the
/// statement is in its place, and only its operand is searched.
fn statement_try(statement: &Stmt, linear: bool) -> Option<(SourceSpan, String)> {
    let expression = "an expression";
    if linear && !matches!(statement_value(statement), Some(Expr::Try { .. })) {
        let nested = "a conditional or a loop nested in the arm of a conditional value";
        if let Some(found) = statement_any_try(statement, nested) {
            return Some(found);
        }
    }
    match statement {
        Stmt::OwnerDecl { initializer: value, .. } => root_try(value, true),
        Stmt::Expression { expression: value, .. } | Stmt::Return { value: Some(value), .. } => {
            root_try(value, false)
        }
        Stmt::Assignment { target, value, .. } => {
            place_try(target).or_else(|| root_try(value, true))
        }
        Stmt::CompoundAssignment { target, value, .. } => {
            place_try(target).or_else(|| expression_try(value, expression))
        }
        Stmt::If { condition, then_branch, else_branch, .. } => {
            expression_try(condition, expression)
                .or_else(|| block_try(then_branch, false))
                .or_else(|| else_branch.as_ref().and_then(|branch| branch_try(branch, false)))
        }
        Stmt::Loop(block) | Stmt::Block(block) => block_try(block, false),
        Stmt::ForRange { start, end, body, .. } => expression_try(start, expression)
            .or_else(|| expression_try(end, expression))
            .or_else(|| block_try(body, false)),
        Stmt::ForArray { collection, body, .. } => {
            expression_try(collection, expression).or_else(|| block_try(body, false))
        }
        Stmt::Return { value: None, .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Drop { .. } => None,
    }
}

/// The value of `statement` when it has one.
fn statement_value(statement: &Stmt) -> Option<&Expr> {
    match statement {
        Stmt::OwnerDecl { initializer: value, .. }
        | Stmt::Expression { expression: value, .. }
        | Stmt::Return { value: Some(value), .. }
        | Stmt::Assignment { value, .. }
        | Stmt::CompoundAssignment { value, .. } => Some(value),
        _ => None,
    }
}

/// Any `?` of `statement`, which stands in `place`.
fn statement_any_try(statement: &Stmt, place: &str) -> Option<(SourceSpan, String)> {
    let span = super::conditional_tries::statement_try_span(statement)?;
    Some((span, place.to_owned()))
}

/// A misplaced `?` of the whole value `value` of a statement; `linear` holds
/// when the arms of a conditional value are lowered statement by statement.
fn root_try(value: &Expr, linear: bool) -> Option<(SourceSpan, String)> {
    match value {
        Expr::Try { expression, .. } => expression_try(expression, "an expression"),
        Expr::If { condition, then_branch, else_branch, .. } => {
            expression_try(condition, "an expression")
                .or_else(|| block_try(then_branch, linear))
                .or_else(|| else_branch.as_ref().and_then(|branch| branch_try(branch, linear)))
        }
        Expr::Case { subject, branches, .. } => expression_try(subject, "an expression")
            .or_else(|| branches.iter().find_map(|branch| case_branch_try(branch, linear))),
        _ => expression_try(value, "an expression"),
    }
}

fn place_try(place: &Place) -> Option<(SourceSpan, String)> {
    match place {
        Place::Binding { .. } => None,
        Place::Field { object, .. } => place_try(object),
        Place::Index { target, index, .. } => {
            place_try(target).or_else(|| expression_try(index, "the index of an assigned element"))
        }
    }
}

fn branch_try(branch: &IfBranch, linear: bool) -> Option<(SourceSpan, String)> {
    match branch {
        IfBranch::Block(block) => block_try(block, linear),
        IfBranch::ElseIf(nested) => root_try(nested, linear),
    }
}

fn case_branch_try(branch: &CaseBranch, linear: bool) -> Option<(SourceSpan, String)> {
    let guard = branch.guard.as_deref();
    guard.and_then(|guard| expression_try(guard, "a `case` guard")).or_else(|| match &branch.body {
        CaseBody::Expression(value) => expression_try(value, "an expression"),
        CaseBody::Block(block) => block_try(block, linear),
    })
}

/// The first `?` inside `expression`, which stands in `place`; the arms of a
/// conditional are blocks whose statements may hold a `?` of their own.
fn expression_try(expression: &Expr, place: &str) -> Option<(SourceSpan, String)> {
    let arguments_of = |name: &str| format!("an argument of `{name}`");
    match expression {
        Expr::Try { span, .. } => Some((*span, place.to_owned())),
        Expr::Call { callee, arguments, .. } => {
            let constructor = callee.starts_with(|first: char| first.is_ascii_uppercase());
            let place = if constructor { place.to_owned() } else { arguments_of(callee) };
            arguments.iter().find_map(|argument| expression_try(&argument.expression, &place))
        }
        Expr::MethodCall { receiver, method, arguments, .. } => {
            let constructor = super::nested_tries::names_a_type(receiver);
            let place = if constructor { place.to_owned() } else { arguments_of(method) };
            expression_try(receiver, &place).or_else(|| {
                arguments.iter().find_map(|argument| expression_try(&argument.expression, &place))
            })
        }
        Expr::StructLit { fields, .. } => {
            fields.iter().find_map(|field| expression_try(&field.value, place))
        }
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::FieldAccess { object: expression, .. }
        | Expr::BufferLiteral { length: expression, .. } => expression_try(expression, place),
        Expr::Binary { left, right, .. } | Expr::Index { target: left, index: right, .. } => {
            expression_try(left, place).or_else(|| expression_try(right, place))
        }
        Expr::If { .. } | Expr::Case { .. } => root_try(expression, true),
        _ => None,
    }
}
