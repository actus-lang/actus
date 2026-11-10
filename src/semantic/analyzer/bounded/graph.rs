use std::collections::{HashMap, HashSet};

use crate::ast::{Block, Expr, IfBranch, Program, Stmt, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

type CallGraph = HashMap<String, Vec<(String, SourceSpan)>>;

impl Analyzer {
    pub(crate) fn validate_bounded_recursion(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        let graph = call_graph(program);
        for declaration in &program.declarations {
            match declaration {
                TopLevelDecl::Verb(verb) if is_bounded(&verb.metadata) => {
                    visit_graph(&verb.name, &graph, &mut HashSet::new(), recursion_error)?;
                }
                TopLevelDecl::Perform(perform) if !perform.module_import => {
                    for method in &perform.methods {
                        if is_bounded(&method.metadata) {
                            let node = performance_node(perform, &method.name);
                            visit_graph(&node, &graph, &mut HashSet::new(), recursion_error)?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn validate_stack_limit_recursion(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        let graph = call_graph(program);
        for declaration in &program.declarations {
            match declaration {
                TopLevelDecl::Verb(verb) if is_stack_limited(&verb.metadata) => {
                    visit_graph(
                        &verb.name,
                        &graph,
                        &mut HashSet::new(),
                        stack_limit_recursion_error,
                    )?;
                }
                TopLevelDecl::Perform(perform) if !perform.module_import => {
                    for method in &perform.methods {
                        if is_stack_limited(&method.metadata) {
                            let node = performance_node(perform, &method.name);
                            visit_graph(
                                &node,
                                &graph,
                                &mut HashSet::new(),
                                stack_limit_recursion_error,
                            )?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn validate_stack_limit_native_claims(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        let graph = call_graph(program);
        let (external_claims, imported_claims) = external_stack_claims(program);
        for root in stack_limit_roots(program) {
            visit_native_claims(
                &root,
                &graph,
                &external_claims,
                &imported_claims,
                &mut HashSet::new(),
            )?;
        }
        Ok(())
    }

    pub(crate) fn validate_stack_limit_dynamic_dispatch(
        &self,
        program: &Program,
    ) -> Result<(), SemanticError> {
        let graph = unresolved_call_graph(program);
        let performance_nodes =
            graph.keys().filter(|name| name.starts_with("perform::")).cloned().collect::<Vec<_>>();
        for root in stack_limit_roots(program) {
            visit_dynamic_targets(&root, &graph, &performance_nodes, &mut HashSet::new())?;
        }
        Ok(())
    }
}

fn is_bounded(metadata: &[crate::ast::MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, crate::ast::MetaAttribute::Bounded(_)))
}

fn is_stack_limited(metadata: &[crate::ast::MetaAttribute]) -> bool {
    metadata.iter().any(|attribute| matches!(attribute, crate::ast::MetaAttribute::StackLimit(_)))
}

fn stack_limit_roots(program: &Program) -> Vec<String> {
    program
        .declarations
        .iter()
        .flat_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) if is_stack_limited(&verb.metadata) => {
                vec![verb.name.clone()]
            }
            TopLevelDecl::Perform(perform) if !perform.module_import => perform
                .methods
                .iter()
                .filter(|method| is_stack_limited(&method.metadata))
                .map(|method| performance_node(perform, &method.name))
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

fn external_stack_claims(program: &Program) -> (HashMap<String, bool>, HashMap<String, bool>) {
    let mut native = HashMap::new();
    let mut imported = HashMap::new();
    for declaration in &program.declarations {
        let TopLevelDecl::ExternalVerb(verb) = declaration else { continue };
        if verb.unsafe_boundary {
            native.insert(verb.name.clone(), is_stack_limited(&verb.metadata));
        } else if verb.module_import {
            imported.insert(verb.name.clone(), is_stack_limited(&verb.metadata));
        }
    }
    (native, imported)
}

fn visit_native_claims(
    name: &str,
    graph: &CallGraph,
    external_claims: &HashMap<String, bool>,
    imported_claims: &HashMap<String, bool>,
    visited: &mut HashSet<String>,
) -> Result<(), SemanticError> {
    if !visited.insert(name.to_owned()) {
        return Ok(());
    }
    for (callee, span) in graph.get(name).into_iter().flatten() {
        if external_claims.get(callee).is_some_and(|claimed| !claimed) {
            return Err(SemanticError {
                kind: SemanticErrorKind::MissingStackLimitClaim { callee: callee.clone() },
                span: *span,
            });
        }
        if imported_claims.get(callee).is_some_and(|claimed| !claimed) {
            return Err(SemanticError {
                kind: SemanticErrorKind::MissingImportedStackLimitClaim { callee: callee.clone() },
                span: *span,
            });
        }
        if graph.contains_key(callee) {
            visit_native_claims(callee, graph, external_claims, imported_claims, visited)?;
        }
    }
    Ok(())
}

fn call_graph(program: &Program) -> CallGraph {
    resolve_method_edges(unresolved_call_graph(program))
}

fn unresolved_call_graph(program: &Program) -> CallGraph {
    let mut graph = HashMap::new();
    for declaration in &program.declarations {
        match declaration {
            TopLevelDecl::Verb(verb) => {
                graph.insert(verb.name.clone(), block_calls(&verb.body, None));
            }
            TopLevelDecl::Perform(perform) if !perform.module_import => {
                for method in &perform.methods {
                    let node = performance_node(perform, &method.name);
                    graph.insert(node.clone(), block_calls(&method.body, Some(node_prefix(&node))));
                }
            }
            _ => {}
        }
    }
    graph
}

fn resolve_method_edges(graph: CallGraph) -> CallGraph {
    let performance_nodes =
        graph.keys().filter(|name| name.starts_with("perform::")).cloned().collect::<Vec<_>>();
    graph
        .into_iter()
        .map(|(caller, calls)| {
            let calls = calls
                .into_iter()
                .flat_map(|(callee, span)| {
                    if let Some(method) = callee.strip_prefix("@method::") {
                        performance_nodes
                            .iter()
                            .filter(|node| node.ends_with(&format!("::{method}")))
                            .cloned()
                            .map(|node| (node, span))
                            .collect::<Vec<_>>()
                    } else {
                        vec![(callee, span)]
                    }
                })
                .collect();
            (caller, calls)
        })
        .collect()
}

fn visit_dynamic_targets(
    name: &str,
    graph: &CallGraph,
    performance_nodes: &[String],
    visited: &mut HashSet<String>,
) -> Result<(), SemanticError> {
    if !visited.insert(name.to_owned()) {
        return Ok(());
    }
    for (callee, span) in graph.get(name).into_iter().flatten() {
        if let Some(method) = callee.strip_prefix("@method::")
            && !performance_nodes.iter().any(|node| node.ends_with(&format!("::{method}")))
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::OpenStackLimitDispatch { method: method.to_owned() },
                span: *span,
            });
        }
        if graph.contains_key(callee) {
            visit_dynamic_targets(callee, graph, performance_nodes, visited)?;
        }
    }
    Ok(())
}

fn performance_node(perform: &crate::ast::PerformDecl, method: &str) -> String {
    format!("perform::{}::{}::{method}", perform.role_name, perform.target.name)
}

fn node_prefix(node: &str) -> &str {
    node.rsplit_once("::").map_or(node, |(prefix, _)| prefix)
}

fn visit_graph(
    name: &str,
    graph: &CallGraph,
    active: &mut HashSet<String>,
    recursion_error: fn(SourceSpan) -> SemanticError,
) -> Result<(), SemanticError> {
    if !active.insert(name.to_owned()) {
        return Err(recursion_error(SourceSpan::new(0, 0)));
    }
    for (callee, span) in graph.get(name).into_iter().flatten() {
        if graph.contains_key(callee) {
            if active.contains(callee) {
                return Err(recursion_error(*span));
            }
            visit_graph(callee, graph, active, recursion_error)?;
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

fn stack_limit_recursion_error(span: SourceSpan) -> SemanticError {
    SemanticError { kind: SemanticErrorKind::StackLimitRecursion, span }
}

fn block_calls(block: &Block, performance_prefix: Option<&str>) -> Vec<(String, SourceSpan)> {
    block
        .statements
        .iter()
        .flat_map(|statement| statement_calls(statement, performance_prefix))
        .collect()
}

fn statement_calls(
    statement: &Stmt,
    performance_prefix: Option<&str>,
) -> Vec<(String, SourceSpan)> {
    match statement {
        Stmt::OwnerDecl { initializer, .. } => expression_calls(initializer, performance_prefix),
        Stmt::Assignment { value, .. } | Stmt::CompoundAssignment { value, .. } => {
            expression_calls(value, performance_prefix)
        }
        Stmt::Expression { expression, .. } => expression_calls(expression, performance_prefix),
        Stmt::Return { value, .. } => value
            .as_ref()
            .map_or_else(Vec::new, |value| expression_calls(value, performance_prefix)),
        Stmt::If { condition, then_branch, else_branch, .. } => {
            conditional_calls(condition, then_branch, else_branch, performance_prefix)
        }
        Stmt::Loop(block) | Stmt::Block(block) => block_calls(block, performance_prefix),
        Stmt::ForRange { start, end, body, .. } => expression_calls(start, performance_prefix)
            .into_iter()
            .chain(expression_calls(end, performance_prefix))
            .chain(block_calls(body, performance_prefix))
            .collect(),
        Stmt::ForArray { collection, body, .. } => expression_calls(collection, performance_prefix)
            .into_iter()
            .chain(block_calls(body, performance_prefix))
            .collect(),
        Stmt::Drop { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => Vec::new(),
    }
}

fn conditional_calls(
    condition: &Expr,
    then_branch: &Block,
    else_branch: &Option<IfBranch>,
    performance_prefix: Option<&str>,
) -> Vec<(String, SourceSpan)> {
    let mut calls = expression_calls(condition, performance_prefix);
    calls.extend(block_calls(then_branch, performance_prefix));
    if let Some(IfBranch::Block(block)) = else_branch {
        calls.extend(block_calls(block, performance_prefix));
    }
    calls
}

fn expression_calls(
    expression: &Expr,
    performance_prefix: Option<&str>,
) -> Vec<(String, SourceSpan)> {
    match expression {
        Expr::Call { callee, arguments, span } => {
            call_expression_calls(callee, arguments, *span, performance_prefix)
        }
        Expr::MethodCall { receiver, method, arguments, span } => {
            method_expression_calls(receiver, method, arguments, *span, performance_prefix)
        }
        Expr::If { condition, then_branch, else_branch, .. } => {
            conditional_calls(condition, then_branch, else_branch, performance_prefix)
        }
        Expr::Case { subject, branches, .. } => {
            case_expression_calls(subject, branches, performance_prefix)
        }
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. }
        | Expr::Cast { expression, .. }
        | Expr::FieldAccess { object: expression, .. } => {
            expression_calls(expression, performance_prefix)
        }
        Expr::Binary { left, right, .. } => expression_calls(left, performance_prefix)
            .into_iter()
            .chain(expression_calls(right, performance_prefix))
            .collect(),
        Expr::Index { target, index, .. } => expression_calls(target, performance_prefix)
            .into_iter()
            .chain(expression_calls(index, performance_prefix))
            .collect(),
        Expr::StructLit { fields, .. } => fields
            .iter()
            .flat_map(|field| expression_calls(&field.value, performance_prefix))
            .collect(),
        _ => Vec::new(),
    }
}

fn call_expression_calls(
    callee: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    performance_prefix: Option<&str>,
) -> Vec<(String, SourceSpan)> {
    std::iter::once((callee.to_owned(), span))
        .chain(
            arguments
                .iter()
                .flat_map(|argument| expression_calls(&argument.expression, performance_prefix)),
        )
        .collect()
}

fn method_expression_calls(
    receiver: &Expr,
    method: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    performance_prefix: Option<&str>,
) -> Vec<(String, SourceSpan)> {
    let callee = performance_prefix
        .map(|prefix| format!("{prefix}::{method}"))
        .unwrap_or_else(|| format!("@method::{method}"));
    std::iter::once((callee, span))
        .chain(expression_calls(receiver, performance_prefix))
        .chain(
            arguments
                .iter()
                .flat_map(|argument| expression_calls(&argument.expression, performance_prefix)),
        )
        .collect()
}

fn case_expression_calls(
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    performance_prefix: Option<&str>,
) -> Vec<(String, SourceSpan)> {
    let mut calls = expression_calls(subject, performance_prefix);
    for branch in branches {
        match &branch.body {
            crate::ast::CaseBody::Expression(expression) => {
                calls.extend(expression_calls(expression, performance_prefix))
            }
            crate::ast::CaseBody::Block(block) => {
                calls.extend(block_calls(block, performance_prefix))
            }
        }
    }
    calls
}
