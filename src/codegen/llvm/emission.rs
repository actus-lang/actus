use std::path::Path;

use super::super::{BackendEmitError, BackendModule};
use super::verification::{print_module, verify_module};
use crate::target::TargetSpec;

/// Emits the supported Actus backend-neutral contract slice as LLVM IR.
///
/// The current slice covers validated scalar and unit signatures, parameter mapping, basic
/// blocks, direct returns, direct calls, conditional branches, jumps, terminating traps,
/// compile-time end-borrow cleanup boundaries, descriptor-driven Buffer/Region runtime cleanup
/// calls, and regular enum failures with explicit layout metadata. Niche-pointer `Option::None`
/// and pointer-payload `Option::Some` failures use the private pointer representation after
/// semantic lowering. Enum cleanup bridges and full semantic cleanup schedule integration remain
/// open.
pub fn emit_module(module: &BackendModule) -> Result<String, BackendEmitError> {
    let (_context, owned_module) = lower_module(module)?;
    verify_module(owned_module.raw)?;
    print_module(owned_module.raw)
}

/// Emits a validated backend module as a target-specific object file.
///
/// The target is selected from the compiler's `TargetSpec`; LLVM receives the same triple and
/// target data layout before code generation. LLVM writes the requested output path directly; a
/// failed emission returns an error and is never reported as a successful object. This API
/// currently initializes LLVM's native target registry, so hosted object emission is supported
/// for the compiler host target while cross-target registration is added by the target backend
/// boundary.
pub fn emit_object(
    module: &BackendModule,
    target: &TargetSpec,
    output: &Path,
) -> Result<(), BackendEmitError> {
    let (_context, owned_module) = lower_module(module)?;
    verify_module(owned_module.raw)?;
    super::object::emit_object(owned_module.raw, target, output)?;
    Ok(())
}

/// Emits a validated backend module as an owned target-specific object buffer.
pub fn emit_object_bytes(
    module: &BackendModule,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    let (_context, owned_module) = lower_module(module)?;
    verify_module(owned_module.raw)?;
    super::object::emit_object_bytes(owned_module.raw, target)
}

use super::module::lower_module;
