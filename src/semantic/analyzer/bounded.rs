use crate::ast::{Block, BoundedContract, Expr, IfBranch, Stmt, VerbDecl};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

#[derive(Clone, Copy)]
struct Cost {
    iterations: Option<u64>,
    calls: u64,
    call_depth: u64,
    allocations: Option<u64>,
    allocation_bytes: Option<u64>,
    io_bytes: Option<u64>,
    stack_bytes: Option<u64>,
}

impl Cost {
    fn sequence(self, other: Self) -> Self {
        Self {
            iterations: self
                .iterations
                .and_then(|left| other.iterations.map(|right| left.saturating_add(right))),
            calls: self.calls.saturating_add(other.calls),
            call_depth: self.call_depth.max(other.call_depth),
            allocations: add_known(self.allocations, other.allocations),
            allocation_bytes: add_known(self.allocation_bytes, other.allocation_bytes),
            io_bytes: add_known(self.io_bytes, other.io_bytes),
            stack_bytes: add_known(self.stack_bytes, other.stack_bytes),
        }
    }
}

impl Analyzer {
    pub(super) fn validate_bounded_contract(&self, verb: &VerbDecl) -> Result<(), SemanticError> {
        let Some(contract) = bounded_metadata(&verb.metadata) else { return Ok(()) };
        let cost = self.bounded_block_cost(&verb.body)?;
        validate_limit(
            "max_iterations",
            contract.max_iterations.map(u64::from),
            cost.iterations,
            verb.span,
        )?;
        validate_limit(
            "max_calls",
            contract.max_calls.map(u64::from),
            Some(cost.calls),
            verb.span,
        )?;
        validate_limit(
            "max_call_depth",
            contract.max_call_depth.map(u64::from),
            Some(cost.call_depth),
            verb.span,
        )?;
        validate_limit(
            "max_allocations",
            contract.max_allocations.map(u64::from),
            cost.allocations,
            verb.span,
        )?;
        validate_limit(
            "max_allocation_bytes",
            contract.max_allocation_bytes,
            cost.allocation_bytes,
            verb.span,
        )?;
        validate_limit("max_stack_bytes", contract.max_stack_bytes, cost.stack_bytes, verb.span)?;
        validate_limit("max_io_bytes", contract.max_io_bytes, cost.io_bytes, verb.span)?;
        Ok(())
    }

    fn bounded_block_cost(&self, block: &Block) -> Result<Cost, SemanticError> {
        block.statements.iter().try_fold(empty_cost(), |cost, statement| {
            Ok(cost.sequence(self.bounded_statement_cost(statement)?))
        })
    }

    fn bounded_statement_cost(&self, statement: &Stmt) -> Result<Cost, SemanticError> {
        match statement {
            Stmt::Loop(block) => {
                Ok(Cost { iterations: None, ..empty_cost() }
                    .sequence(self.bounded_block_cost(block)?))
            }
            Stmt::ForRange { start, end, body, span, .. } => {
                self.range_cost(start, end, body, *span)
            }
            Stmt::ForArray { collection, body, span, .. } => {
                self.array_cost(collection, body, *span)
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                let condition = self.bounded_expr_cost(condition)?;
                let then_cost = self.bounded_block_cost(then_branch)?;
                let else_cost = else_branch_cost(self, else_branch)?;
                Ok(condition.sequence(select_branch(then_cost, else_cost)))
            }
            Stmt::OwnerDecl { initializer, ty, .. } => {
                let cost = self.bounded_expr_cost(initializer)?;
                Ok(cost.sequence(Cost {
                    stack_bytes: ty.as_deref().and_then(type_size),
                    ..empty_cost()
                }))
            }
            Stmt::Assignment { value, .. } | Stmt::CompoundAssignment { value, .. } => {
                self.bounded_expr_cost(value)
            }
            Stmt::Expression { expression, .. } => self.bounded_expr_cost(expression),
            Stmt::Return { value, .. } => {
                value.as_ref().map_or(Ok(empty_cost()), |value| self.bounded_expr_cost(value))
            }
            Stmt::Block(block) => self.bounded_block_cost(block),
            Stmt::Drop { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => Ok(empty_cost()),
        }
    }

    fn range_cost(
        &self,
        start: &Expr,
        end: &Expr,
        body: &Block,
        span: SourceSpan,
    ) -> Result<Cost, SemanticError> {
        let Some(count) = range_count(start, end) else { return Ok(unknown_loop(span)) };
        let body_cost = self.bounded_block_cost(body)?;
        Ok(Cost {
            iterations: body_cost
                .iterations
                .map(|value| count.saturating_add(value.saturating_mul(count))),
            calls: body_cost.calls.saturating_mul(count),
            call_depth: body_cost.call_depth,
            allocations: scale_known(body_cost.allocations, count),
            allocation_bytes: scale_known(body_cost.allocation_bytes, count),
            io_bytes: scale_known(body_cost.io_bytes, count),
            stack_bytes: body_cost.stack_bytes,
        })
    }

    fn array_cost(
        &self,
        collection: &Expr,
        body: &Block,
        span: SourceSpan,
    ) -> Result<Cost, SemanticError> {
        let Some(type_name) = self.expression_type_name(collection) else {
            return Ok(unknown_loop(span));
        };
        let Some(count) = array_capacity(&type_name) else { return Ok(unknown_loop(span)) };
        let body_cost = self.bounded_block_cost(body)?;
        Ok(Cost {
            iterations: body_cost
                .iterations
                .map(|value| count.saturating_add(value.saturating_mul(count))),
            calls: body_cost.calls.saturating_mul(count),
            call_depth: body_cost.call_depth,
            allocations: scale_known(body_cost.allocations, count),
            allocation_bytes: scale_known(body_cost.allocation_bytes, count),
            io_bytes: scale_known(body_cost.io_bytes, count),
            stack_bytes: body_cost.stack_bytes,
        })
    }

    fn bounded_expr_cost(&self, expression: &Expr) -> Result<Cost, SemanticError> {
        let cost = match expression {
            Expr::Call { callee, arguments, .. } => {
                call_cost(self, callee, arguments_cost(self, arguments)?)
            }
            Expr::MethodCall { receiver, arguments, .. } => call_cost(
                self,
                "<method>",
                self.bounded_expr_cost(receiver)?.sequence(arguments_cost(self, arguments)?),
            ),
            Expr::If { condition, then_branch, else_branch, .. } => {
                let condition_cost = self.bounded_expr_cost(condition)?;
                let then_cost = self.bounded_block_cost(then_branch)?;
                let else_cost = else_branch_cost(self, else_branch)?;
                condition_cost.sequence(select_branch(then_cost, else_cost))
            }
            Expr::Case { subject, branches, .. } => {
                let subject = self.bounded_expr_cost(subject)?;
                let branches = branches.iter().try_fold(empty_cost(), |cost, branch| {
                    let branch_cost = match &branch.body {
                        crate::ast::CaseBody::Expression(expression) => {
                            self.bounded_expr_cost(expression)?
                        }
                        crate::ast::CaseBody::Block(block) => self.bounded_block_cost(block)?,
                    };
                    Ok(select_branch(cost, branch_cost))
                })?;
                subject.sequence(branches)
            }
            Expr::Grouping { expression, .. }
            | Expr::Unary { expression, .. }
            | Expr::Borrow { expression, .. }
            | Expr::Try { expression, .. }
            | Expr::Cast { expression, .. } => self.bounded_expr_cost(expression)?,
            Expr::Binary { left, right, .. } => {
                self.bounded_expr_cost(left)?.sequence(self.bounded_expr_cost(right)?)
            }
            Expr::Index { target, index, .. } => {
                self.bounded_expr_cost(target)?.sequence(self.bounded_expr_cost(index)?)
            }
            Expr::FieldAccess { object, .. } => self.bounded_expr_cost(object)?,
            Expr::StructLit { fields, .. } => {
                fields.iter().try_fold(empty_cost(), |cost, field| {
                    Ok(cost.sequence(self.bounded_expr_cost(&field.value)?))
                })?
            }
            Expr::BufferLiteral { length, .. } => Cost {
                allocations: Some(1),
                allocation_bytes: integer_value(length),
                ..empty_cost()
            },
            _ => empty_cost(),
        };
        Ok(cost)
    }
}

fn bounded_metadata(metadata: &[crate::ast::MetaAttribute]) -> Option<&BoundedContract> {
    metadata.iter().find_map(|attribute| match attribute {
        crate::ast::MetaAttribute::Bounded(contract) => Some(contract),
        _ => None,
    })
}

fn empty_cost() -> Cost {
    Cost {
        iterations: Some(0),
        calls: 0,
        call_depth: 0,
        allocations: Some(0),
        allocation_bytes: Some(0),
        io_bytes: Some(0),
        stack_bytes: Some(0),
    }
}

fn unknown_loop(span: SourceSpan) -> Cost {
    let _ = span;
    Cost { iterations: None, ..empty_cost() }
}

fn max_known(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        _ => None,
    }
}

fn add_known(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    left.and_then(|left| right.map(|right| left.saturating_add(right)))
}

fn scale_known(value: Option<u64>, factor: u64) -> Option<u64> {
    value.map(|value| value.saturating_mul(factor))
}

fn select_branch(left: Cost, right: Cost) -> Cost {
    Cost {
        iterations: max_known(left.iterations, right.iterations),
        calls: left.calls.max(right.calls),
        call_depth: left.call_depth.max(right.call_depth),
        allocations: max_known(left.allocations, right.allocations),
        allocation_bytes: max_known(left.allocation_bytes, right.allocation_bytes),
        io_bytes: max_known(left.io_bytes, right.io_bytes),
        stack_bytes: max_known(left.stack_bytes, right.stack_bytes),
    }
}

fn else_branch_cost(analyzer: &Analyzer, branch: &Option<IfBranch>) -> Result<Cost, SemanticError> {
    match branch {
        Some(IfBranch::Block(block)) => analyzer.bounded_block_cost(block),
        Some(IfBranch::ElseIf(expression)) => analyzer.bounded_expr_cost(expression),
        None => Ok(empty_cost()),
    }
}

fn arguments_cost(
    analyzer: &Analyzer,
    arguments: &[crate::ast::Argument],
) -> Result<Cost, SemanticError> {
    arguments.iter().try_fold(empty_cost(), |cost, argument| {
        Ok(cost.sequence(analyzer.bounded_expr_cost(&argument.expression)?))
    })
}

fn call_cost(analyzer: &Analyzer, callee: &str, arguments: Cost) -> Cost {
    let Some(signature) = analyzer.signatures.get(callee) else {
        return Cost {
            calls: arguments.calls.saturating_add(1),
            call_depth: arguments.call_depth.saturating_add(1),
            ..arguments
        };
    };
    let allocation = signature.effects.contains(&crate::ast::Effect::Allocation);
    let io = signature.effects.iter().any(|effect| {
        matches!(effect, crate::ast::Effect::Filesystem | crate::ast::Effect::Network)
    });
    Cost {
        iterations: arguments.iterations,
        calls: arguments.calls.saturating_add(1),
        call_depth: arguments.call_depth.saturating_add(1),
        allocations: if allocation {
            arguments.allocations.map(|value| value.saturating_add(1))
        } else {
            arguments.allocations
        },
        allocation_bytes: allocation.then_some(None).flatten().or(arguments.allocation_bytes),
        io_bytes: io.then_some(None).flatten().or(arguments.io_bytes),
        stack_bytes: arguments.stack_bytes,
    }
}

fn validate_limit(
    name: &str,
    limit: Option<u64>,
    required: Option<u64>,
    span: SourceSpan,
) -> Result<(), SemanticError> {
    let Some(limit) = limit else { return Ok(()) };
    let Some(required) = required else {
        return Err(bounded_error(name, limit, "an unbounded path", span));
    };
    if required > limit {
        return Err(bounded_error(name, limit, &required.to_string(), span));
    }
    Ok(())
}

fn bounded_error(dimension: &str, limit: u64, required: &str, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::BoundedBudgetExceeded {
            dimension: dimension.to_owned(),
            limit,
            required: required.to_owned(),
        },
        span,
    }
}

fn range_count(start: &Expr, end: &Expr) -> Option<u64> {
    let (start, end) = (integer_value(start)?, integer_value(end)?);
    Some(end.saturating_sub(start))
}

fn integer_value(expression: &Expr) -> Option<u64> {
    match expression {
        Expr::Integer { value, .. } => value.parse().ok(),
        _ => None,
    }
}

fn array_capacity(type_name: &str) -> Option<u64> {
    type_name.strip_prefix("Array[")?.split(',').nth(1)?.trim_end_matches(']').parse().ok()
}

fn type_size(type_name: &str) -> Option<u64> {
    match type_name {
        "Bool" | "u8" | "i8" => Some(1),
        "u16" | "i16" => Some(2),
        "u32" | "i32" | "f32" => Some(4),
        "u64" | "i64" | "f64" => Some(8),
        "Int" => Some(8),
        _ => None,
    }
}
