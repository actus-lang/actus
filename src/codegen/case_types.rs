use std::collections::HashMap;

use crate::ast::{CaseBody, CaseBranch};

use super::case_payload::add_payload_types;
use super::expressions::initializer_type;
use super::layout::LayoutRegistry;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

pub(super) fn infer_case_type(
    branches: &[CaseBranch],
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let mut inferred = None;
    let mut has_block_body = false;
    for branch in branches {
        let candidate = match &branch.body {
            CaseBody::Expression(expression) => {
                Some(initializer_type(expression, types, functions, layouts)?)
            }
            CaseBody::Block(_) => {
                has_block_body = true;
                None
            }
        };
        if let Some(candidate) = candidate {
            if let Some(expected) = inferred {
                if expected != candidate {
                    return Err(NativeEmitError(
                        "case branches have different native types".to_owned(),
                    ));
                }
            } else {
                inferred = Some(candidate);
            }
        }
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
        let candidate = match &branch.body {
            CaseBody::Expression(expression) => {
                Some(initializer_type(expression, &branch_types, functions, layouts)?)
            }
            CaseBody::Block(_) => {
                has_block_body = true;
                None
            }
        };
        if let Some(candidate) = candidate {
            if let Some(expected) = inferred {
                if expected != candidate {
                    return Err(NativeEmitError(
                        "case branches have different native types".to_owned(),
                    ));
                }
            } else {
                inferred = Some(candidate);
            }
        }
    }
    finish_case_type(inferred, has_block_body)
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
