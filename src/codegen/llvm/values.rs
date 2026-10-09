use llvm_sys::core::{
    LLVMInt1TypeInContext, LLVMInt8TypeInContext, LLVMInt16TypeInContext, LLVMInt32TypeInContext,
    LLVMInt64TypeInContext, LLVMVoidTypeInContext,
};
use llvm_sys::prelude::{LLVMContextRef, LLVMTypeRef};

use super::super::{BackendEmitError, BackendValueType};

pub(super) fn llvm_type(
    context: LLVMContextRef,
    ty: BackendValueType,
) -> Result<LLVMTypeRef, BackendEmitError> {
    let result = match ty {
        BackendValueType::Unit => unsafe { LLVMVoidTypeInContext(context) },
        BackendValueType::Bool => unsafe { LLVMInt1TypeInContext(context) },
        BackendValueType::Integer { width: 8, .. } => unsafe { LLVMInt8TypeInContext(context) },
        BackendValueType::Integer { width: 16, .. } => unsafe { LLVMInt16TypeInContext(context) },
        BackendValueType::Integer { width: 32, .. } => unsafe { LLVMInt32TypeInContext(context) },
        BackendValueType::Integer { width: 64, .. } => unsafe { LLVMInt64TypeInContext(context) },
        BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Pointer
        | BackendValueType::Aggregate { .. }
        | BackendValueType::Region { .. } => {
            return Err(BackendEmitError("LLVM type lowering is not implemented".to_owned()));
        }
    };
    Ok(result)
}
