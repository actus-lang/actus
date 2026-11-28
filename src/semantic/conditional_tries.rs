//! The arms of a conditional that holds a `?` (bug #130). An arm of an `if` or
//! a `case` runs only when it is selected, so a `?` inside it cannot be moved
//! in front of the conditional (`nested_tries`). Instead the arm is made a
//! block whose `?`s are statements of their own: an arm written as an
//! expression becomes a block, the value of the arm is bound to a generated
//! `erg` local when it holds a `?` (`{ erg v = load()?; v }`), and the block is
//! then normalized like any other block, which rewrites the `?`s nested in that
//! value. An `else if` whose condition or arms hold a `?` becomes an `else`
//! block holding the inner `if`, so its condition is evaluated, and its `?`
//! returns early, only when the earlier conditions failed.
//!
//! The lowering gives every such statement `?` its early return with the exit
//! list the analyzer planned for it (ADR-0125), on both back ends.

use crate::ast::{Argument, Block, CaseBody, CaseBranch, Expr, IfBranch, Stmt};
use crate::lexer::SourceSpan;

/// Prepares the arms of the conditional that is the whole value of
/// `statement` (an `erg` initializer, a returned value, an assigned value or
/// an expression statement) when one of them holds a `?`, and returns the arm
/// blocks, which the caller normalizes as blocks.
///
/// The arms of a value an owner receives (`erg x = if ..`, `x = case ..`)
/// are lowered statement by statement into the block that ends with the
/// value, so their value is bound when it is a `?`. The arms of a returned
/// value or of a `case` statement are lowered as blocks of their own; their
/// statements are only normalized, and a conditional written as an arm is
/// prepared in the same way.
pub(super) fn value_arms<'a>(
    statement: &'a mut Stmt,
    fresh_name: &mut dyn FnMut() -> String,
) -> Vec<&'a mut Block> {
    let (root, linear) = match statement {
        Stmt::OwnerDecl { initializer: value, .. } | Stmt::Assignment { value, .. } => {
            (value, true)
        }
        Stmt::Return { value: Some(value), .. } | Stmt::Expression { expression: value, .. } => {
            (value, false)
        }
        _ => return Vec::new(),
    };
    let mut arms = Vec::new();
    if expression_has_try(root) {
        conditional_arms(root, linear, fresh_name, &mut arms);
    }
    arms
}

/// Collects the arm blocks of the conditional `root` into `arms`, preparing
/// them as `value_arms` describes.
fn conditional_arms<'a>(
    root: &'a mut Expr,
    linear: bool,
    fresh_name: &mut dyn FnMut() -> String,
    arms: &mut Vec<&'a mut Block>,
) {
    match root {
        Expr::If { then_branch, else_branch, .. } => {
            if linear {
                split_else_if(else_branch);
            }
            arms.push(prepare_arm(then_branch, linear, fresh_name));
            match else_branch {
                Some(IfBranch::Block(block)) => arms.push(prepare_arm(block, linear, fresh_name)),
                Some(IfBranch::ElseIf(nested)) => {
                    conditional_arms(nested, linear, fresh_name, arms)
                }
                None => {}
            }
        }
        Expr::Case { branches, .. } => {
            for branch in branches {
                if let CaseBody::Expression(value) = &mut branch.body
                    && expression_has_try(value)
                {
                    if !linear && matches!(value.as_ref(), Expr::If { .. } | Expr::Case { .. }) {
                        let CaseBody::Expression(value) = &mut branch.body else { unreachable!() };
                        conditional_arms(value, linear, fresh_name, arms);
                        continue;
                    }
                    let span = super::nested_tries::expression_span(value);
                    let expression = value.as_ref().clone();
                    branch.body = CaseBody::Block(Block {
                        statements: vec![Stmt::Expression { expression, span }],
                        span,
                    });
                }
                if let CaseBody::Block(block) = &mut branch.body {
                    arms.push(prepare_arm(block, linear, fresh_name));
                }
            }
        }
        _ => {}
    }
}

/// An arm block as it is normalized: with its value bound when the arm is
/// lowered statement by statement.
fn prepare_arm<'a>(
    block: &'a mut Block,
    linear: bool,
    fresh_name: &mut dyn FnMut() -> String,
) -> &'a mut Block {
    if linear { bind_tail(block, fresh_name) } else { block }
}

/// Binds the value of the arm `block`, its tail expression, to a generated
/// local when it is a `?` or a conditional with a `?` on one of its paths, so
/// the `?` is a statement of the arm. A `?` nested deeper in the value is
/// bound when the arm is normalized, and the value stays the tail.
fn bind_tail<'a>(block: &'a mut Block, fresh_name: &mut dyn FnMut() -> String) -> &'a mut Block {
    if let Some(Stmt::Expression { expression, span }) = block.statements.last()
        && (matches!(expression, Expr::Try { .. })
            || super::nested_tries::has_conditional_try(expression))
    {
        let (value, span) = (expression.clone(), *span);
        block.statements.pop();
        let mut prefix = Vec::new();
        let read = super::nested_tries::bind(value, span, fresh_name, &mut prefix);
        block.statements.extend(prefix);
        block.statements.push(Stmt::Expression { expression: read, span });
    }
    block
}

/// Turns `else if c { .. }` into `else { if c { .. } }` when the inner `if`
/// holds a `?` anywhere, so the inner `if` is a statement of a block that is
/// normalized like any other.
pub(super) fn split_else_if(else_branch: &mut Option<IfBranch>) {
    let Some(IfBranch::ElseIf(nested)) = else_branch else { return };
    if !expression_has_try(nested) {
        return;
    }
    let Expr::If { condition, then_branch, else_branch: inner, span } = nested.as_ref().clone()
    else {
        return;
    };
    let statement = Stmt::If { condition: *condition, then_branch, else_branch: inner, span };
    *else_branch = Some(IfBranch::Block(Block { statements: vec![statement], span }));
}

/// Whether a `?` appears anywhere in `expression`, on any path, including the
/// arms of a conditional and the arguments of a call.
pub(super) fn expression_has_try(expression: &Expr) -> bool {
    expression_try_span(expression).is_some()
}

/// Whether a `?` appears anywhere in the statements of `block`.
pub(super) fn block_has_try(block: &Block) -> bool {
    block.statements.iter().find_map(statement_try_span).is_some()
}

/// Whether a `?` appears anywhere in the `else` branch `branch`.
pub(super) fn branch_has_try(branch: &IfBranch) -> bool {
    branch_try_span(branch).is_some()
}

/// Whether a `?` appears anywhere in the guard or the body of `branch`.
pub(super) fn case_branch_has_try(branch: &CaseBranch) -> bool {
    case_branch_try_span(branch).is_some()
}

/// The span of the first `?` of `expression`, on any path.
fn expression_try_span(expression: &Expr) -> Option<SourceSpan> {
    let each = |arguments: &[Argument]| {
        arguments.iter().find_map(|argument| expression_try_span(&argument.expression))
    };
    match expression {
        Expr::Try { span, .. } => Some(*span),
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::FieldAccess { object: expression, .. }
        | Expr::BufferLiteral { length: expression, .. } => expression_try_span(expression),
        Expr::Binary { left, right, .. } | Expr::Index { target: left, index: right, .. } => {
            expression_try_span(left).or_else(|| expression_try_span(right))
        }
        Expr::Call { arguments, .. } => each(arguments),
        Expr::MethodCall { receiver, arguments, .. } => {
            expression_try_span(receiver).or_else(|| each(arguments))
        }
        Expr::StructLit { fields, .. } => {
            fields.iter().find_map(|field| expression_try_span(&field.value))
        }
        Expr::If { condition, then_branch, else_branch, .. } => expression_try_span(condition)
            .or_else(|| then_branch.statements.iter().find_map(statement_try_span))
            .or_else(|| else_branch.as_ref().and_then(branch_try_span)),
        Expr::Case { subject, branches, .. } => {
            expression_try_span(subject).or_else(|| branches.iter().find_map(case_branch_try_span))
        }
        _ => None,
    }
}

fn branch_try_span(branch: &IfBranch) -> Option<SourceSpan> {
    match branch {
        IfBranch::Block(block) => block.statements.iter().find_map(statement_try_span),
        IfBranch::ElseIf(nested) => expression_try_span(nested),
    }
}

fn case_branch_try_span(branch: &CaseBranch) -> Option<SourceSpan> {
    branch.guard.as_deref().and_then(expression_try_span).or_else(|| match &branch.body {
        CaseBody::Expression(expression) => expression_try_span(expression),
        CaseBody::Block(block) => block.statements.iter().find_map(statement_try_span),
    })
}

/// The span of the first `?` of `statement`, in any position.
pub(super) fn statement_try_span(statement: &Stmt) -> Option<SourceSpan> {
    let block = |block: &Block| block.statements.iter().find_map(statement_try_span);
    match statement {
        Stmt::OwnerDecl { initializer: value, .. }
        | Stmt::Assignment { value, .. }
        | Stmt::CompoundAssignment { value, .. }
        | Stmt::Expression { expression: value, .. }
        | Stmt::Return { value: Some(value), .. } => expression_try_span(value),
        Stmt::If { condition, then_branch, else_branch, .. } => expression_try_span(condition)
            .or_else(|| block(then_branch))
            .or_else(|| else_branch.as_ref().and_then(branch_try_span)),
        Stmt::Loop(body) | Stmt::Block(body) => block(body),
        Stmt::ForRange { start, end, body, .. } => {
            expression_try_span(start).or_else(|| expression_try_span(end)).or_else(|| block(body))
        }
        Stmt::ForArray { collection, body, .. } => {
            expression_try_span(collection).or_else(|| block(body))
        }
        Stmt::Return { value: None, .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Drop { .. } => None,
    }
}
