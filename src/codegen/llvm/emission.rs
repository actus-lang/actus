use std::path::Path;

use super::super::{BackendEmitError, BackendModule};
use super::module::lower_module;
use super::verification::{print_module, verify_module};
use crate::target::TargetSpec;

/// Emits the supported Actus backend-neutral contract slice as LLVM IR.
pub fn emit_module(module: &BackendModule) -> Result<String, BackendEmitError> {
    let (_context, owned_module) = lower_module(module, usize::BITS as u8)?;
    verify_module(owned_module.raw)?;
    print_module(owned_module.raw)
}

/// Emits a validated backend-neutral module as a target-specific object file.
pub fn emit_object(
    module: &BackendModule,
    target: &TargetSpec,
    output: &Path,
) -> Result<(), BackendEmitError> {
    let (_context, owned_module) = lower_module(module, target.pointer_width.bits())?;
    verify_module(owned_module.raw)?;
    super::object::emit_object(owned_module.raw, target, output)?;
    Ok(())
}

/// Emits a validated backend-neutral module as an owned target-specific object buffer.
pub fn emit_object_bytes(
    module: &BackendModule,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    let (_context, owned_module) = lower_module(module, target.pointer_width.bits())?;
    verify_module(owned_module.raw)?;
    super::object::emit_object_bytes(owned_module.raw, target)
}
