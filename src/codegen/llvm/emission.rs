use std::path::Path;

use super::super::{BackendEmitError, BackendModule};
use super::module::lower_module;
use super::verification::{print_module, verify_module, verify_no_float_ir};
use crate::target::TargetSpec;

/// Explicit LLVM object-emission controls. Both options are disabled by default
/// so ordinary object output keeps its existing representation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ObjectEmissionOptions {
    pub emit_stack_size_section: bool,
    pub function_sections: bool,
    pub optimization_level: ObjectOptimizationLevel,
}

/// LLVM optimization level used by an object emission request (ADR-0114).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ObjectOptimizationLevel {
    /// No IR pipeline; code generation at `None`. The `debug` profile.
    #[default]
    None,
    /// No IR pipeline; code generation at `Default`. Used by stack-frame
    /// evidence fixtures that measure code generation alone.
    Default,
    /// `default<O3>` IR pipeline; code generation at `Aggressive`. The
    /// `release` profile.
    Speed,
    /// `default<O2>` IR pipeline over functions marked `minsize` and
    /// `optsize`, which is how LLVM optimizes for size; code generation at
    /// `Default`. The `size` profile.
    Size,
}

impl ObjectOptimizationLevel {
    /// The level a configured build optimization level selects.
    pub const fn from_configuration(level: crate::configuration::OptimizationLevel) -> Self {
        match level {
            crate::configuration::OptimizationLevel::None => Self::None,
            crate::configuration::OptimizationLevel::Speed => Self::Speed,
            crate::configuration::OptimizationLevel::Size => Self::Size,
        }
    }

    /// The target machine code generation level of this level.
    pub(super) const fn code_generation_level(
        self,
    ) -> llvm_sys::target_machine::LLVMCodeGenOptLevel {
        use llvm_sys::target_machine::LLVMCodeGenOptLevel;
        match self {
            Self::None => LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
            Self::Default | Self::Size => LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            Self::Speed => LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
        }
    }

    /// The new-pass-manager pipeline this level runs before code generation.
    pub(super) const fn pipeline(self) -> Option<&'static std::ffi::CStr> {
        match self {
            Self::None | Self::Default => None,
            Self::Speed => Some(c"default<O3>"),
            Self::Size => Some(c"default<O2>"),
        }
    }
}

/// Emits the supported Actus backend-neutral contract slice as LLVM IR.
pub fn emit_module(module: &BackendModule) -> Result<String, BackendEmitError> {
    validate_integer_only_signature(module)?;
    let (_context, owned_module) =
        lower_module(module, usize::BITS as u8, crate::target::EntryContract::Hosted)?;
    verify_module(owned_module.raw)?;
    print_module(owned_module.raw)
}

fn validate_integer_only_signature(module: &BackendModule) -> Result<(), BackendEmitError> {
    for function in &module.functions {
        reject_float_signature(function.symbol.as_str(), function.return_value.ty)?;
        for parameter in &function.parameters {
            reject_float_signature(function.symbol.as_str(), parameter.ty)?;
        }
    }
    for external in &module.externals {
        reject_float_signature(external.symbol.as_str(), external.return_value.ty)?;
        for parameter in &external.parameters {
            reject_float_signature(external.symbol.as_str(), parameter.ty)?;
        }
    }
    Ok(())
}

fn reject_float_signature(
    symbol: &str,
    ty: super::super::BackendValueType,
) -> Result<(), BackendEmitError> {
    if matches!(ty, super::super::BackendValueType::Float { .. }) {
        return Err(BackendEmitError(format!(
            "LLVM integer-only emission rejects floating-point type in `{symbol}`"
        )));
    }
    Ok(())
}

/// Emits a validated backend-neutral module as a target-specific object file.
pub fn emit_object(
    module: &BackendModule,
    target: &TargetSpec,
    output: &Path,
) -> Result<(), BackendEmitError> {
    emit_object_with_configuration(module, target, output, true)
}

/// Emits a validated backend-neutral module using the requested relocation model.
pub fn emit_object_with_configuration(
    module: &BackendModule,
    target: &TargetSpec,
    output: &Path,
    position_independent: bool,
) -> Result<(), BackendEmitError> {
    emit_object_with_configuration_and_options(
        module,
        target,
        output,
        position_independent,
        ObjectEmissionOptions::default(),
    )
}

/// Emits an object with explicit target-machine section controls.
pub fn emit_object_with_configuration_and_options(
    module: &BackendModule,
    target: &TargetSpec,
    output: &Path,
    position_independent: bool,
    options: ObjectEmissionOptions,
) -> Result<(), BackendEmitError> {
    let (_context, owned_module) =
        lower_module(module, target.pointer_width.bits(), target.entry_contract())?;
    verify_module(owned_module.raw)?;
    super::object::emit_object(owned_module.raw, target, output, position_independent, options)?;
    Ok(())
}

/// Emits a validated backend-neutral module as an owned target-specific object buffer.
pub fn emit_object_bytes(
    module: &BackendModule,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    emit_object_bytes_with_configuration(module, target, true)
}

/// Emits a validated backend-neutral object buffer using the requested relocation model.
pub fn emit_object_bytes_with_configuration(
    module: &BackendModule,
    target: &TargetSpec,
    position_independent: bool,
) -> Result<Vec<u8>, BackendEmitError> {
    emit_object_bytes_with_configuration_and_float_policy(
        module,
        target,
        position_independent,
        false,
    )
}

/// Emits an object buffer and optionally rejects floating-point LLVM IR.
pub fn emit_object_bytes_with_configuration_and_float_policy(
    module: &BackendModule,
    target: &TargetSpec,
    position_independent: bool,
    verify_no_float: bool,
) -> Result<Vec<u8>, BackendEmitError> {
    emit_object_bytes_with_configuration_and_options_and_float_policy(
        module,
        target,
        position_independent,
        verify_no_float,
        ObjectEmissionOptions::default(),
    )
}

/// Emits an object buffer with explicit target-machine section controls.
pub fn emit_object_bytes_with_configuration_and_options_and_float_policy(
    module: &BackendModule,
    target: &TargetSpec,
    position_independent: bool,
    verify_no_float: bool,
    options: ObjectEmissionOptions,
) -> Result<Vec<u8>, BackendEmitError> {
    emit_object_bytes_with_local_symbols(
        module,
        &std::collections::BTreeSet::new(),
        target,
        position_independent,
        verify_no_float,
        options,
    )
}

/// Emits an object buffer whose `local_symbols` have internal linkage
/// (ADR-0112).
pub fn emit_object_bytes_with_local_symbols(
    module: &BackendModule,
    local_symbols: &std::collections::BTreeSet<String>,
    target: &TargetSpec,
    position_independent: bool,
    verify_no_float: bool,
    options: ObjectEmissionOptions,
) -> Result<Vec<u8>, BackendEmitError> {
    let (_context, owned_module) =
        lower_module(module, target.pointer_width.bits(), target.entry_contract())?;
    super::linkage::localize(owned_module.raw, local_symbols)?;
    verify_module(owned_module.raw)?;
    if verify_no_float {
        verify_no_float_ir(owned_module.raw)?;
    }
    super::object::emit_object_bytes(owned_module.raw, target, position_independent, options)
}
