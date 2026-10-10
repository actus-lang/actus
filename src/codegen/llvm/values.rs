use llvm_sys::core::{
    LLVMArrayType2, LLVMInt1TypeInContext, LLVMInt8TypeInContext, LLVMInt16TypeInContext,
    LLVMInt32TypeInContext, LLVMInt64TypeInContext, LLVMPointerTypeInContext,
    LLVMVoidTypeInContext,
};
use llvm_sys::prelude::{LLVMContextRef, LLVMTypeRef};

use super::super::{BackendEmitError, BackendLayout, BackendValueType};

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
        BackendValueType::Pointer => unsafe { LLVMPointerTypeInContext(context, 0) },
        BackendValueType::NichePointerOption { .. } => unsafe {
            LLVMPointerTypeInContext(context, 0)
        },
        BackendValueType::Aggregate { layout } | BackendValueType::Region { layout } => {
            return aggregate_type(context, layout);
        }
        BackendValueType::Integer { .. } | BackendValueType::Float { .. } => {
            return Err(BackendEmitError("LLVM type lowering is not implemented".to_owned()));
        }
    };
    Ok(result)
}

fn aggregate_type(
    context: LLVMContextRef,
    layout: BackendLayout,
) -> Result<LLVMTypeRef, BackendEmitError> {
    let element = match layout.alignment {
        1 => unsafe { LLVMInt8TypeInContext(context) },
        2 => unsafe { LLVMInt16TypeInContext(context) },
        4 => unsafe { LLVMInt32TypeInContext(context) },
        8 => unsafe { LLVMInt64TypeInContext(context) },
        _ => {
            return Err(BackendEmitError(
                "LLVM direct aggregate alignment must be 1, 2, 4, or 8 bytes".to_owned(),
            ));
        }
    };
    let element_count = u64::from(layout.size / layout.alignment);
    Ok(unsafe { LLVMArrayType2(element, element_count) })
}
