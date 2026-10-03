use std::collections::{HashMap, HashSet, VecDeque};

use crate::ast::{Block, Expr, ExternalVerbDecl, Place, Stmt, VerbDecl};

use super::NativeEmitError;

/// Selects the native declarations reachable from an object root.
///
/// Visibility is deliberately not consulted here: semantic module aggregation
/// has already established which declarations are legal. This pass only keeps
/// private implementation helpers reachable from a legal public root while
/// retaining external bridges as declarations.
pub(super) fn reachable_declarations<'a>(
    verbs: &[&'a VerbDecl],
    external_verbs: &[&'a ExternalVerbDecl],
    symbol: Option<&str>,
) -> Result<(Vec<&'a VerbDecl>, Vec<&'a ExternalVerbDecl>), NativeEmitError> {
    let local = verbs.iter().map(|verb| (verb.name.as_str(), *verb)).collect::<HashMap<_, _>>();
    let external =
        external_verbs.iter().map(|verb| (verb.name.as_str(), *verb)).collect::<HashMap<_, _>>();
    let roots = root_names(verbs, symbol)?;
    let mut queue = VecDeque::from(roots);
    let mut visited = HashSet::new();
    while let Some(name) = queue.pop_front() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if let Some(verb) = local.get(name.as_str()) {
            let mut calls = Vec::new();
            collect_block_calls(&verb.body, &mut calls);
            queue.extend(calls.into_iter().filter(|call| {
                local.contains_key(call.as_str()) || external.contains_key(call.as_str())
            }));
        }
    }
    let selected_verbs =
        verbs.iter().copied().filter(|verb| visited.contains(verb.name.as_str())).collect();
    // External bridges describe symbols supplied by another native object or
    // the runtime. Keep the complete declaration set available to lowering;
    // unlike Actus verbs, these declarations do not emit function bodies.
    let selected_external = external_verbs.to_vec();
    Ok((selected_verbs, selected_external))
}

fn root_names(verbs: &[&VerbDecl], symbol: Option<&str>) -> Result<Vec<String>, NativeEmitError> {
    if let Some(symbol) = symbol {
        if !verbs.iter().any(|verb| verb.name == symbol) {
            return Err(NativeEmitError(format!("entry verb `{symbol}` was not found")));
        }
        return Ok(vec![symbol.to_owned()]);
    }
    let public =
        verbs.iter().filter(|verb| verb.is_open).map(|verb| verb.name.clone()).collect::<Vec<_>>();
    if public.is_empty() {
        return Ok(verbs.iter().map(|verb| verb.name.clone()).collect());
    }
    Ok(public)
}

fn collect_block_calls(block: &Block, calls: &mut Vec<String>) {
    for statement in &block.statements {
        collect_statement_calls(statement, calls);
    }
}

fn collect_statement_calls(statement: &Stmt, calls: &mut Vec<String>) {
    match statement {
        Stmt::OwnerDecl { initializer, .. } | Stmt::Expression { expression: initializer, .. } => {
            collect_expression_calls(initializer, calls)
        }
        Stmt::Assignment { target, value, .. } | Stmt::CompoundAssignment { target, value, .. } => {
            collect_place_calls(target, calls);
            collect_expression_calls(value, calls);
        }
        Stmt::If { condition, then_branch, else_branch, .. } => {
            collect_expression_calls(condition, calls);
            collect_block_calls(then_branch, calls);
            if let Some(branch) = else_branch {
                match branch {
                    crate::ast::IfBranch::Block(block) => collect_block_calls(block, calls),
                    crate::ast::IfBranch::ElseIf(expression) => {
                        collect_expression_calls(expression, calls)
                    }
                }
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                collect_expression_calls(value, calls);
            }
        }
        Stmt::Loop(block) | Stmt::Block(block) => collect_block_calls(block, calls),
        Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Drop { .. } => {}
    }
}

fn collect_place_calls(place: &Place, calls: &mut Vec<String>) {
    match place {
        Place::Binding { .. } => {}
        Place::Field { object, .. } => collect_place_calls(object, calls),
        Place::Index { target, index, .. } => {
            collect_place_calls(target, calls);
            collect_expression_calls(index, calls);
        }
    }
}

fn collect_expression_calls(expression: &Expr, calls: &mut Vec<String>) {
    match expression {
        Expr::Call { callee, arguments, .. } => {
            calls.push(
                callee.split_once('[').map_or_else(|| callee.clone(), |(name, _)| name.to_owned()),
            );
            for argument in arguments {
                collect_expression_calls(&argument.expression, calls);
            }
        }
        Expr::MethodCall { receiver, method, arguments, .. } => {
            calls.push(method.clone());
            collect_expression_calls(receiver, calls);
            for argument in arguments {
                collect_expression_calls(&argument.expression, calls);
            }
        }
        Expr::Grouping { expression, .. }
        | Expr::Unary { expression, .. }
        | Expr::Borrow { expression, .. }
        | Expr::Try { expression, .. }
        | Expr::Cast { expression, .. } => collect_expression_calls(expression, calls),
        Expr::Binary { left, right, .. } => {
            collect_expression_calls(left, calls);
            collect_expression_calls(right, calls);
        }
        Expr::StructLit { fields, .. } => {
            for field in fields {
                collect_expression_calls(&field.value, calls);
            }
        }
        Expr::FieldAccess { object, .. } => collect_expression_calls(object, calls),
        Expr::Index { target, index, .. } => {
            collect_expression_calls(target, calls);
            collect_expression_calls(index, calls);
        }
        Expr::Case { subject, branches, .. } => collect_case_calls(subject, branches, calls),
        Expr::If { condition, then_branch, else_branch, .. } => {
            collect_expression_calls(condition, calls);
            collect_block_calls(then_branch, calls);
            if let Some(crate::ast::IfBranch::Block(block)) = else_branch {
                collect_block_calls(block, calls);
            }
        }
        Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BoolLiteral { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => {}
    }
}

fn collect_case_calls(
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    calls: &mut Vec<String>,
) {
    collect_expression_calls(subject, calls);
    for branch in branches {
        if let Some(guard) = &branch.guard {
            collect_expression_calls(guard, calls);
        }
        match &branch.body {
            crate::ast::CaseBody::Expression(expression) => {
                collect_expression_calls(expression, calls)
            }
            crate::ast::CaseBody::Block(block) => collect_block_calls(block, calls),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::reachable_declarations;
    use crate::ast::TopLevelDecl;
    use crate::lexer::scan;
    use crate::parser::parse;

    #[test]
    fn keeps_reachable_private_helpers_and_drops_unreachable_verbs() {
        let source = "verb main() -> Int { return helper(); } verb helper() -> Int { return leaf(); } verb leaf() -> Int { return 7; } verb unused() -> Int { return 9; }";
        let (tokens, errors) = scan(source);
        assert!(errors.is_empty());
        let program = parse(tokens).expect("dependency fixture should parse");
        let verbs = program
            .declarations
            .iter()
            .filter_map(|declaration| match declaration {
                TopLevelDecl::Verb(verb) => Some(verb),
                _ => None,
            })
            .collect::<Vec<_>>();
        let (selected, external) =
            reachable_declarations(&verbs, &[], Some("main")).expect("root should resolve");
        assert!(external.is_empty());
        assert_eq!(
            selected.iter().map(|verb| verb.name.as_str()).collect::<Vec<_>>(),
            ["main", "helper", "leaf"]
        );
    }
}
