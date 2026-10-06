use std::collections::HashMap;

use crate::ast::{CaseBody, CaseBranch};

use super::super::expressions::initializer_type;
use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::payload::add_payload_types;

pub(in crate::codegen) fn infer_case_type(
    branches: &[CaseBranch],
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let mut inferred = None;
    let mut has_block_body = false;
    for branch in branches {
        let (candidate, is_block) = body_type(&branch.body, types, functions, layouts, true)?;
        has_block_body |= is_block;
        inferred = merge_case_type(inferred, candidate)?;
    }
    finish_case_type(inferred, has_block_body)
}

pub(super) fn branch_type(
    branches: &[CaseBranch],
    subject_type: NativeType,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    value_producing: bool,
) -> Result<NativeType, NativeEmitError> {
    let mut inferred = None;
    let mut has_block_body = false;
    for branch in branches {
        let mut branch_types = local_types.clone();
        add_payload_types(branch, subject_type, &mut branch_types, layouts);
        let (candidate, is_block) =
            body_type(&branch.body, &branch_types, functions, layouts, value_producing)?;
        has_block_body |= is_block;
        inferred = merge_case_type(inferred, candidate)?;
    }
    finish_case_type(inferred, has_block_body)
}

fn body_type(
    body: &CaseBody,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    value_producing: bool,
) -> Result<(Option<NativeType>, bool), NativeEmitError> {
    match body {
        CaseBody::Expression(expression) => {
            Ok((Some(initializer_type(expression, types, functions, layouts)?), false))
        }
        CaseBody::Block(block) => {
            Ok((block_type(block, types, functions, layouts, value_producing)?, true))
        }
    }
}

fn block_type(
    block: &crate::ast::Block,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    value_producing: bool,
) -> Result<Option<NativeType>, NativeEmitError> {
    if value_producing {
        block_type_for_value(block, types, functions, layouts)
    } else {
        block_type_for_statement(block, types, functions, layouts)
    }
}

fn block_type_for_statement(
    block: &crate::ast::Block,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<Option<NativeType>, NativeEmitError> {
    let Some(statement) = block.statements.last() else { return Ok(None) };
    match statement {
        crate::ast::Stmt::Expression { expression, span }
            if span.end == expression_end(expression) =>
        {
            Ok(Some(initializer_type(expression, types, functions, layouts)?))
        }
        crate::ast::Stmt::If { then_branch, else_branch, .. } => {
            let expression = synthetic_if(then_branch, else_branch);
            Ok(Some(initializer_type(&expression, types, functions, layouts)?))
        }
        _ => Ok(None),
    }
}

fn block_type_for_value(
    block: &crate::ast::Block,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<Option<NativeType>, NativeEmitError> {
    let Some((statement, prefix)) = block.statements.split_last() else { return Ok(None) };
    let mut block_types = types.clone();
    for statement in prefix {
        let crate::ast::Stmt::OwnerDecl { name, ty, initializer, .. } = statement else {
            continue;
        };
        let native_type = ty
            .as_deref()
            .and_then(|name| {
                NativeType::from_name(name)
                    .or_else(|| layouts.type_for_name(name))
                    .or_else(|| layouts.array_id(name).map(NativeType::Array))
            })
            .map(Ok)
            .unwrap_or_else(|| initializer_type(initializer, &block_types, functions, layouts));
        block_types.insert(name, native_type?);
    }
    match statement {
        crate::ast::Stmt::Expression { expression, .. } => {
            Ok(Some(initializer_type(expression, &block_types, functions, layouts)?))
        }
        crate::ast::Stmt::If { then_branch, else_branch, .. } => {
            let expression = synthetic_if(then_branch, else_branch);
            Ok(Some(initializer_type(&expression, &block_types, functions, layouts)?))
        }
        _ => Ok(None),
    }
}

fn synthetic_if(
    then_branch: &crate::ast::Block,
    else_branch: &Option<crate::ast::IfBranch>,
) -> crate::ast::Expr {
    crate::ast::Expr::If {
        condition: Box::new(crate::ast::Expr::BoolLiteral {
            value: true,
            span: crate::lexer::SourceSpan::new(0, 0),
        }),
        then_branch: then_branch.clone(),
        else_branch: else_branch.clone(),
        span: crate::lexer::SourceSpan::new(0, 0),
    }
}

fn merge_case_type(
    inferred: Option<NativeType>,
    candidate: Option<NativeType>,
) -> Result<Option<NativeType>, NativeEmitError> {
    let Some(candidate) = candidate else { return Ok(inferred) };
    match inferred {
        Some(expected) if expected != candidate => Err(NativeEmitError(format!(
            "case branches have different native types (expected {expected:?}, found {candidate:?})"
        ))),
        Some(expected) => Ok(Some(expected)),
        None => Ok(Some(candidate)),
    }
}

fn expression_end(expression: &crate::ast::Expr) -> usize {
    use crate::ast::Expr;

    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => span.end,
    }
}

fn finish_case_type(
    inferred: Option<NativeType>,
    has_block_body: bool,
) -> Result<NativeType, NativeEmitError> {
    match (inferred, has_block_body) {
        (Some(result), _) => Ok(result),
        (None, true) => Ok(NativeType::Void),
        (None, false) => Err(NativeEmitError("case has no value-producing branch".to_owned())),
    }
}
