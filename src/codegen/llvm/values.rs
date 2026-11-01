use llvm_sys::core::{
    LLVMArrayType2, LLVMDoubleTypeInContext, LLVMFloatTypeInContext, LLVMInt1TypeInContext,
    LLVMInt8TypeInContext, LLVMInt16TypeInContext, LLVMInt32TypeInContext, LLVMInt64TypeInContext,
    LLVMIntTypeInContext, LLVMPointerTypeInContext, LLVMVoidTypeInContext,
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
        BackendValueType::Integer { width: 1, .. } => unsafe { LLVMInt1TypeInContext(context) },
        BackendValueType::Integer { width: 8, .. } => unsafe { LLVMInt8TypeInContext(context) },
        BackendValueType::Integer { width: 16, .. } => unsafe { LLVMInt16TypeInContext(context) },
        BackendValueType::Integer { width: 32, .. } => unsafe { LLVMInt32TypeInContext(context) },
        BackendValueType::Integer { width: 64, .. } => unsafe { LLVMInt64TypeInContext(context) },
        BackendValueType::Integer { width, .. } if (2..=128).contains(&width) => unsafe {
            LLVMIntTypeInContext(context, u32::from(width))
        },
        BackendValueType::Float { width: 32 } => unsafe { LLVMFloatTypeInContext(context) },
        BackendValueType::Float { width: 64 } => unsafe { LLVMDoubleTypeInContext(context) },
        BackendValueType::Pointer => unsafe { LLVMPointerTypeInContext(context, 0) },
        BackendValueType::NichePointerOption { .. } => unsafe {
            LLVMPointerTypeInContext(context, 0)
        },
        BackendValueType::Region { .. } => unsafe { LLVMPointerTypeInContext(context, 0) },
        BackendValueType::Aggregate { layout }
        | BackendValueType::Enum { layout: super::super::BackendEnumLayout { layout, .. } } => {
            return aggregate_type(context, layout);
        }
        BackendValueType::Integer { width, .. } => {
            return Err(BackendEmitError(format!(
                "LLVM type lowering rejects invalid integer width {width}; expected 1..=128"
            )));
        }
        BackendValueType::Float { width } => {
            return Err(BackendEmitError(format!(
                "LLVM type lowering rejects invalid floating-point width {width}; expected 32 or 64"
            )));
        }
    };
    Ok(result)
}

fn aggregate_type(
    context: LLVMContextRef,
    layout: BackendLayout,
) -> Result<LLVMTypeRef, BackendEmitError> {
    if layout.alignment == 1 {
        let byte = unsafe { LLVMInt8TypeInContext(context) };
        return Ok(unsafe { LLVMArrayType2(byte, u64::from(layout.size)) });
    }
    let element = match layout.alignment {
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
