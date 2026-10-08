use std::collections::{HashMap, HashSet, VecDeque};

use crate::ast::{Block, Expr, ExternalVerbDecl, Place, Stmt, VerbDecl, lookup_call_intrinsic};
use crate::lexer::SourceSpan;

use super::NativeEmitError;

/// Selects the native declarations reachable from an object root.
///
/// Visibility is deliberately not consulted here: semantic module aggregation
/// has already established which declarations are legal. This pass only keeps
/// private implementation helpers reachable from a legal public root while
/// retaining external bridges as declarations.
pub(super) fn reachable_declarations_with_roots<'a>(
    verbs: &[&'a VerbDecl],
    external_verbs: &[&'a ExternalVerbDecl],
    symbol: Option<&str>,
    exported_roots: Option<&[String]>,
) -> Result<(Vec<&'a VerbDecl>, Vec<&'a ExternalVerbDecl>), NativeEmitError> {
    let local = verbs.iter().map(|verb| (verb.name.as_str(), *verb)).collect::<HashMap<_, _>>();
    let external =
        external_verbs.iter().map(|verb| (verb.name.as_str(), *verb)).collect::<HashMap<_, _>>();
    let roots = root_names(verbs, symbol, exported_roots)?;
    let mut queue = VecDeque::from(roots);
    let mut visited = HashSet::new();
    while let Some(name) = queue.pop_front() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if let Some(verb) = local.get(name.as_str()) {
            let mut calls = Vec::new();
            collect_block_calls(&verb.body, &mut calls);
            for call in calls {
                if local.contains_key(call.name.as_str())
                    || external.contains_key(call.name.as_str())
                {
                    queue.push_back(call.name);
                } else if call.requires_resolution && !is_native_builtin(&call.name) {
                    return Err(NativeEmitError(format_unresolved_call(&name, &call)));
                }
            }
        }
    }
    let selected_verbs =
        verbs.iter().copied().filter(|verb| visited.contains(verb.name.as_str())).collect();
    // External bridges describe symbols supplied by another native object or
    // the runtime. Keep only bridges reached by selected Actus verbs so an
    // unused imported module cannot add linker-visible declarations.
    let selected_external = external_verbs
        .iter()
        .copied()
        .filter(|verb| visited.contains(verb.name.as_str()))
        .collect();
    Ok((selected_verbs, selected_external))
}

pub(super) fn reachable_call_names(
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    symbol: Option<&str>,
) -> Result<HashSet<String>, NativeEmitError> {
    let (selected_verbs, _) =
        reachable_declarations_with_roots(verbs, external_verbs, symbol, None)?;
    let mut names = HashSet::new();
    for verb in selected_verbs {
        let mut calls = Vec::new();
        collect_block_calls(&verb.body, &mut calls);
        names.extend(calls.into_iter().map(|call| call.name));
    }
    Ok(names)
}

struct NativeCall {
    name: String,
    span: SourceSpan,
    requires_resolution: bool,
}

fn is_native_builtin(name: &str) -> bool {
    lookup_call_intrinsic(name).is_some()
        || matches!(name, "Array" | "Arena" | "Buffer" | "Ok" | "Err" | "Some" | "None" | "place")
        || name.chars().next().is_some_and(char::is_uppercase)
        || name.starts_with("Array[")
        || name.starts_with("Arena[")
}

fn format_unresolved_call(caller: &str, call: &NativeCall) -> String {
    format!(
        "unresolved native dependency `{}` called from `{}` at {}:{}",
        call.name, caller, call.span.start, call.span.end
    )
}

fn root_names(
    verbs: &[&VerbDecl],
    symbol: Option<&str>,
    exported_roots: Option<&[String]>,
) -> Result<Vec<String>, NativeEmitError> {
    if let Some(symbol) = symbol {
        if !verbs.iter().any(|verb| verb.name == symbol) {
            return Err(NativeEmitError(format!("entry verb `{symbol}` was not found")));
        }
        return Ok(vec![symbol.to_owned()]);
    }
    if let Some(exported_roots) = exported_roots {
        return Ok(exported_roots
            .iter()
            .flat_map(|root| {
                verbs
                    .iter()
                    .filter(move |verb| {
                        verb.name == *root || verb.name.starts_with(&format!("{root}__"))
                    })
                    .map(|verb| verb.name.clone())
            })
            .collect());
    }
    let public =
        verbs.iter().filter(|verb| verb.is_open).map(|verb| verb.name.clone()).collect::<Vec<_>>();
    if public.is_empty() {
        return Ok(verbs.iter().map(|verb| verb.name.clone()).collect());
    }
    Ok(public)
}

fn collect_block_calls(block: &Block, calls: &mut Vec<NativeCall>) {
    for statement in &block.statements {
        collect_statement_calls(statement, calls);
    }
}

fn collect_statement_calls(statement: &Stmt, calls: &mut Vec<NativeCall>) {
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
        Stmt::ForRange { start, end, body, .. } => {
            collect_expression_calls(start, calls);
            collect_expression_calls(end, calls);
            collect_block_calls(body, calls);
        }
        Stmt::ForArray { collection, body, .. } => {
            collect_expression_calls(collection, calls);
            collect_block_calls(body, calls);
        }
        Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Drop { .. } => {}
    }
}

fn collect_place_calls(place: &Place, calls: &mut Vec<NativeCall>) {
    match place {
        Place::Binding { .. } => {}
        Place::Field { object, .. } => collect_place_calls(object, calls),
        Place::Index { target, index, .. } => {
            collect_place_calls(target, calls);
            collect_expression_calls(index, calls);
        }
    }
}

fn collect_expression_calls(expression: &Expr, calls: &mut Vec<NativeCall>) {
    match expression {
        Expr::Call { callee, arguments, span } => {
            collect_direct_call(callee, arguments, *span, calls)
        }
        Expr::MethodCall { receiver, method, arguments, span } => {
            collect_method_call(receiver, method, arguments, *span, calls)
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

fn collect_direct_call(
    callee: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    calls: &mut Vec<NativeCall>,
) {
    calls.push(NativeCall {
        name: callee.split_once('[').map_or_else(|| callee.to_owned(), |(name, _)| name.to_owned()),
        span,
        requires_resolution: true,
    });
    for argument in arguments {
        collect_expression_calls(&argument.expression, calls);
    }
}

fn collect_method_call(
    receiver: &Expr,
    method: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    calls: &mut Vec<NativeCall>,
) {
    calls.push(NativeCall { name: method.to_owned(), span, requires_resolution: false });
    collect_expression_calls(receiver, calls);
    for argument in arguments {
        collect_expression_calls(&argument.expression, calls);
    }
}

fn collect_case_calls(
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    calls: &mut Vec<NativeCall>,
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
    use super::reachable_declarations_with_roots;
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
            reachable_declarations_with_roots(&verbs, &[], Some("main"), None)
                .expect("root should resolve");
        assert!(external.is_empty());
        assert_eq!(
            selected.iter().map(|verb| verb.name.as_str()).collect::<Vec<_>>(),
            ["main", "helper", "leaf"]
        );
    }

    #[test]
    fn rejects_unresolved_native_dependencies_at_the_call_span() {
        let source = "verb main() -> Int { return missing(); }";
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
        let error = reachable_declarations_with_roots(&verbs, &[], Some("main"), None)
            .expect_err("unresolved dependency should fail closed");
        assert!(error.0.contains("unresolved native dependency `missing`"));
        assert!(error.0.contains("called from `main` at 28:37"));
    }
}
