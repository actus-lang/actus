use std::collections::BTreeMap;
use std::ffi::CString;

use llvm_sys::core::LLVMBuildCall2;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendValueId};
use super::declarations::FunctionDeclaration;

pub(super) fn emit_call(
    builder: LLVMBuilderRef,
    callee: &str,
    argument_ids: &[BackendValueId],
    result: Option<BackendValueId>,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let declaration = declarations
        .get(callee)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM callee `{callee}` is unavailable")))?;
    if argument_ids.len() != declaration.parameter_count {
        return Err(BackendEmitError(format!(
            "LLVM call `{callee}` expects {} arguments, got {}",
            declaration.parameter_count,
            argument_ids.len()
        )));
    }
    let mut arguments = argument_ids
        .iter()
        .map(|id| {
            values
                .get(id)
                .copied()
                .ok_or_else(|| BackendEmitError(format!("LLVM call value {} is unavailable", id.0)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let call_name = CString::new(result.map(|id| format!("call{}", id.0)).unwrap_or_default())
        .map_err(|_| BackendEmitError("LLVM call name contains NUL".to_owned()))?;
    let call = unsafe {
        LLVMBuildCall2(
            builder,
            declaration.function_type,
            declaration.value,
            arguments.as_mut_ptr(),
            arguments.len() as u32,
            call_name.as_ptr(),
        )
    };
    if call.is_null() {
        return Err(BackendEmitError("LLVM failed to build call".to_owned()));
    }
    if let Some(result) = result {
        if declaration.returns_void {
            return Err(BackendEmitError("LLVM call result cannot be void".to_owned()));
        }
        values.insert(result, call);
    }
    Ok(())
}
