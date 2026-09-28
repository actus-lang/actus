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
        let (candidate, is_block) = body_type(&branch.body, types, functions, layouts)?;
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
) -> Result<NativeType, NativeEmitError> {
    let mut inferred = None;
    let mut has_block_body = false;
    for branch in branches {
        let mut branch_types = local_types.clone();
        add_payload_types(branch, subject_type, &mut branch_types, layouts);
        let (candidate, is_block) = body_type(&branch.body, &branch_types, functions, layouts)?;
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
) -> Result<(Option<NativeType>, bool), NativeEmitError> {
    match body {
        CaseBody::Expression(expression) => {
            Ok((Some(initializer_type(expression, types, functions, layouts)?), false))
        }
        CaseBody::Block(_) => Ok((None, true)),
    }
}

fn merge_case_type(
    inferred: Option<NativeType>,
    candidate: Option<NativeType>,
) -> Result<Option<NativeType>, NativeEmitError> {
    let Some(candidate) = candidate else { return Ok(inferred) };
    match inferred {
        Some(expected) if expected != candidate => {
            Err(NativeEmitError("case branches have different native types".to_owned()))
        }
        Some(expected) => Ok(Some(expected)),
        None => Ok(Some(candidate)),
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
