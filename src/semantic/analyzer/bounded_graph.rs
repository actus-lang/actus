use std::collections::{HashMap, HashSet};

use crate::ast::{Block, Expr, IfBranch, Program, Stmt, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

type CallGraph = HashMap<String, Vec<(String, SourceSpan)>>;

impl Analyzer {
    pub(super) fn validate_bounded_recursion(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        let graph = call_graph(program);
        for declaration in &program.declarations {
            let TopLevelDecl::Verb(verb) = declaration else { continue };
            if is_bounded(&verb.metadata) {
                visit_graph(&verb.name, &graph, &mut HashSet::new())?;
            }
        }
        Ok(())
    }
}

fn is_bounded(metadata: &[crate::ast::MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, crate::ast::MetaAttribute::Bounded(_)))
}

fn call_graph(program: &Program) -> CallGraph {
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => Some((verb.name.clone(), block_calls(&verb.body))),
            _ => None,
        })
        .collect()
}

fn visit_graph(
    name: &str,
    graph: &CallGraph,
    active: &mut HashSet<String>,
) -> Result<(), SemanticError> {
    if !active.insert(name.to_owned()) {
        return Err(recursion_error(SourceSpan::new(0, 0)));
    }
    for (callee, span) in graph.get(name).into_iter().flatten() {
        if graph.contains_key(callee) {
            if active.contains(callee) {
                return Err(recursion_error(*span));
            }
            visit_graph(callee, graph, active)?;
        }
    }
    active.remove(name);
    Ok(())
}

fn recursion_error(span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::BoundedBudgetExceeded {
            dimension: "max_call_depth".to_owned(),
            limit: 0,
            required: "recursive call path".to_owned(),
        },
        span,
    }
}

fn block_calls(block: &Block) -> Vec<(String, SourceSpan)> {
    block.statements.iter().flat_map(statement_calls).collect()
}

fn statement_calls(statement: &Stmt) -> Vec<(String, SourceSpan)> {
    match statement {
        Stmt::OwnerDecl { initializer, .. } => expression_calls(initializer),
        Stmt::Assignment { value, .. } | Stmt::CompoundAssignment { value, .. } => {
            expression_calls(value)
        }
        Stmt::Expression { expression, .. } => expression_calls(expression),
        Stmt::Return { value, .. } => value.as_ref().map_or_else(Vec::new, expression_calls),
        Stmt::If { condition, then_branch, else_branch, .. } => {
            conditional_calls(condition, then_branch, else_branch)
        }
        Stmt::Loop(block) | Stmt::Block(block) => block_calls(block),
        Stmt::ForRange { start, end, body, .. } => expression_calls(start)
            .into_iter()
            .chain(expression_calls(end))
            .chain(block_calls(body))
            .collect(),
        Stmt::ForArray { collection, body, .. } => {
            expression_calls(collection).into_iter().chain(block_calls(body)).collect()
        }
        Stmt::Drop { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => Vec::new(),
    }
}

fn conditional_calls(
    condition: &Expr,
    then_branch: &Block,
    else_branch: &Option<IfBranch>,
) -> Vec<(String, SourceSpan)> {
    let mut calls = expression_calls(condition);
    calls.extend(block_calls(then_branch));
    if let Some(IfBranch::Block(block)) = else_branch {
        calls.extend(block_calls(block));
    }
    calls
}

fn expression_calls(expression: &Expr) -> Vec<(String, SourceSpan)> {
    match expression {
        Expr::Call { callee, arguments, span } => std::iter::once((callee.clone(), *span))
            .chain(arguments.iter().flat_map(|argument| expression_calls(&argument.expression)))
            .collect(),
        Expr::MethodCall { receiver, arguments, .. } => expression_calls(receiver)
            .into_iter()
            .chain(arguments.iter().flat_map(|argument| expression_calls(&argument.expression)))
            .collect(),
        Expr::If { condition, then_branch, else_branch, .. } => {
            conditional_calls(condition, then_branch, else_branch)
        }
        Expr::Case { subject, branches, .. } => {
            let mut calls = expression_calls(subject);
            for branch in branches {
                match &branch.body {
                    crate::ast::CaseBody::Expression(expression) => {
                        calls.extend(expression_calls(expression))
                    }
                    crate::ast::CaseBody::Block(block) => calls.extend(block_calls(block)),
                }
            }
            calls
        }
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::FieldAccess { object: expression, .. } => expression_calls(expression),
        Expr::Binary { left, right, .. } => {
            expression_calls(left).into_iter().chain(expression_calls(right)).collect()
        }
        Expr::Index { target, index, .. } => {
            expression_calls(target).into_iter().chain(expression_calls(index)).collect()
        }
        Expr::StructLit { fields, .. } => {
            fields.iter().flat_map(|field| expression_calls(&field.value)).collect()
        }
        _ => Vec::new(),
    }
}
