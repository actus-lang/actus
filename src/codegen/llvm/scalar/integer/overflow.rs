//! Trapping and saturating integer arithmetic (ADR-0115).
//!
//! A trapping add, subtract, multiply or negate calls the matching
//! `llvm.{s,u}{add,sub,mul}.with.overflow` intrinsic and branches to
//! `llvm.trap` when its overflow bit is set. Saturating add and subtract call
//! `llvm.{s,u}{add,sub}.sat`; saturating multiply selects the crossed bound
//! from the overflow bit, because LLVM has no saturating multiply.

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMBuildCall2, LLVMBuildCondBr, LLVMBuildExtractValue,
    LLVMBuildICmp, LLVMBuildSelect, LLVMBuildXor, LLVMConstInt, LLVMConstNull, LLVMGetGlobalParent,
    LLVMGetIntTypeWidth, LLVMGetIntrinsicDeclaration, LLVMIntrinsicGetType, LLVMLookupIntrinsicID,
    LLVMPositionBuilderAtEnd, LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMTypeRef, LLVMValueRef};

use super::super::super::super::{BackendEmitError, BackendIntegerBinaryOp};

/// The function and builder an overflow check emits into.
pub(in crate::codegen::llvm) struct OverflowSite {
    pub(in crate::codegen::llvm) context: LLVMContextRef,
    pub(in crate::codegen::llvm) function: LLVMValueRef,
    pub(in crate::codegen::llvm) builder: LLVMBuilderRef,
}

/// `left op right` for a trapping or saturating operation, or `None` for any
/// other operation.
pub(super) fn emit_checked_arithmetic(
    site: &OverflowSite,
    operation: BackendIntegerBinaryOp,
    signed: bool,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> Option<Result<LLVMValueRef, BackendEmitError>> {
    use BackendIntegerBinaryOp::{
        SaturatingAdd, SaturatingMultiply, SaturatingSubtract, TrappingAdd, TrappingMultiply,
        TrappingSubtract,
    };
    let sign = if signed { "s" } else { "u" };
    Some(match operation {
        TrappingAdd | TrappingSubtract | TrappingMultiply => emit_trapping(
            site,
            &format!("llvm.{sign}{}.with.overflow", stem(operation)),
            left,
            right,
        ),
        SaturatingAdd | SaturatingSubtract => {
            call_intrinsic(site, &format!("llvm.{sign}{}.sat", stem(operation)), left, right)
        }
        SaturatingMultiply => emit_saturating_multiply(site, signed, left, right),
        _ => return None,
    })
}

/// `0 - operand`, trapping on `-MIN` and on any nonzero unsigned operand.
pub(super) fn emit_trapping_negate(
    site: &OverflowSite,
    signed: bool,
    operand: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let zero = unsafe { LLVMConstNull(LLVMTypeOf(operand)) };
    let sign = if signed { "s" } else { "u" };
    emit_trapping(site, &format!("llvm.{sign}sub.with.overflow"), zero, operand)
}

fn stem(operation: BackendIntegerBinaryOp) -> &'static str {
    match operation {
        BackendIntegerBinaryOp::TrappingAdd | BackendIntegerBinaryOp::SaturatingAdd => "add",
        BackendIntegerBinaryOp::TrappingSubtract | BackendIntegerBinaryOp::SaturatingSubtract => {
            "sub"
        }
        _ => "mul",
    }
}

fn emit_trapping(
    site: &OverflowSite,
    intrinsic: &str,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let (value, overflow) = with_overflow(site, intrinsic, left, right)?;
    let ok_block = unsafe {
        LLVMAppendBasicBlockInContext(site.context, site.function, c"actus.overflow.ok".as_ptr())
    };
    let trap_block = unsafe {
        LLVMAppendBasicBlockInContext(site.context, site.function, c"actus.overflow.trap".as_ptr())
    };
    if ok_block.is_null() || trap_block.is_null() {
        return Err(BackendEmitError("LLVM failed to create overflow-guard blocks".to_owned()));
    }
    if unsafe { LLVMBuildCondBr(site.builder, overflow, trap_block, ok_block) }.is_null() {
        return Err(BackendEmitError("LLVM failed to emit overflow-guard branch".to_owned()));
    }
    unsafe { LLVMPositionBuilderAtEnd(site.builder, trap_block) };
    super::super::super::guards::emit_llvm_trap(site.context, site.function, site.builder)?;
    unsafe { LLVMPositionBuilderAtEnd(site.builder, ok_block) };
    Ok(value)
}

fn emit_saturating_multiply(
    site: &OverflowSite,
    signed: bool,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let sign = if signed { "s" } else { "u" };
    let (value, overflow) =
        with_overflow(site, &format!("llvm.{sign}mul.with.overflow"), left, right)?;
    let ty = unsafe { LLVMTypeOf(value) };
    let width = unsafe { LLVMGetIntTypeWidth(ty) };
    let bound = if signed {
        let maximum = unsafe { LLVMConstInt(ty, (1u64 << (width - 1)) - 1, 0) };
        let minimum = unsafe { LLVMConstInt(ty, 1u64 << (width - 1), 0) };
        let signs = unsafe { LLVMBuildXor(site.builder, left, right, c"actus.sat.signs".as_ptr()) };
        let negative = unsafe {
            LLVMBuildICmp(
                site.builder,
                LLVMIntPredicate::LLVMIntSLT,
                signs,
                LLVMConstNull(ty),
                c"actus.sat.negative".as_ptr(),
            )
        };
        unsafe {
            LLVMBuildSelect(site.builder, negative, minimum, maximum, c"actus.sat.bound".as_ptr())
        }
    } else {
        unsafe { llvm_sys::core::LLVMConstAllOnes(ty) }
    };
    let result =
        unsafe { LLVMBuildSelect(site.builder, overflow, bound, value, c"actus.sat.mul".as_ptr()) };
    (!result.is_null())
        .then_some(result)
        .ok_or_else(|| BackendEmitError("LLVM failed to emit saturating multiply".to_owned()))
}

/// The wrapped value and the overflow bit of a `*.with.overflow` call.
fn with_overflow(
    site: &OverflowSite,
    intrinsic: &str,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> Result<(LLVMValueRef, LLVMValueRef), BackendEmitError> {
    let pair = call_intrinsic(site, intrinsic, left, right)?;
    let value = unsafe { LLVMBuildExtractValue(site.builder, pair, 0, c"actus.wrapped".as_ptr()) };
    let overflow =
        unsafe { LLVMBuildExtractValue(site.builder, pair, 1, c"actus.overflow".as_ptr()) };
    if value.is_null() || overflow.is_null() {
        return Err(BackendEmitError(format!("LLVM failed to read the result of `{intrinsic}`")));
    }
    Ok((value, overflow))
}

/// Calls the integer intrinsic `name`, overloaded on the operand type.
fn call_intrinsic(
    site: &OverflowSite,
    name: &str,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let mut overloads: [LLVMTypeRef; 1] = [unsafe { LLVMTypeOf(left) }];
    let id = unsafe { LLVMLookupIntrinsicID(name.as_ptr().cast(), name.len()) };
    if id == 0 {
        return Err(BackendEmitError(format!("LLVM intrinsic `{name}` is unavailable")));
    }
    let module = unsafe { LLVMGetGlobalParent(site.function) };
    let declaration =
        unsafe { LLVMGetIntrinsicDeclaration(module, id, overloads.as_mut_ptr(), overloads.len()) };
    let function_type =
        unsafe { LLVMIntrinsicGetType(site.context, id, overloads.as_mut_ptr(), overloads.len()) };
    let mut arguments = [left, right];
    let call = unsafe {
        LLVMBuildCall2(
            site.builder,
            function_type,
            declaration,
            arguments.as_mut_ptr(),
            arguments.len() as u32,
            c"actus.arith".as_ptr(),
        )
    };
    (!call.is_null())
        .then_some(call)
        .ok_or_else(|| BackendEmitError(format!("LLVM failed to call `{name}`")))
}
