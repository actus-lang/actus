use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use super::super::{BackendEmitError, BackendInstruction, BackendModule};
use super::verification::{print_module, verify_module};
use crate::target::TargetSpec;

/// Keeps only the functions reachable from a configured root entry.
///
/// Root object emission must not expose unrelated verbs. This mirrors the
/// object reachability contract used by the other native backend while keeping
/// the policy inside the LLVM backend implementation.
pub(crate) fn retain_reachable_root_functions(
    mut module: BackendModule,
    entry_symbol: &str,
) -> BackendModule {
    let functions = module
        .functions
        .iter()
        .map(|function| (function.symbol.as_str().to_owned(), function))
        .collect::<BTreeMap<_, _>>();
    let mut reachable = BTreeSet::new();
    let mut pending = VecDeque::from([entry_symbol.to_owned()]);
    while let Some(symbol) = pending.pop_front() {
        if !reachable.insert(symbol.clone()) {
            continue;
        }
        let Some(function) = functions.get(&symbol) else { continue };
        for block in &function.blocks {
            for instruction in &block.instructions {
                let BackendInstruction::Call { callee, .. } = instruction else { continue };
                let callee = callee.as_str();
                if functions.contains_key(callee) {
                    pending.push_back(callee.to_owned());
                }
            }
        }
    }
    module.functions.retain(|function| reachable.contains(function.symbol.as_str()));
    module
}

/// Emits the supported Actus backend-neutral contract slice as LLVM IR.
///
/// The current slice covers validated scalar and unit signatures, parameter mapping, basic
/// blocks, direct returns, direct calls, conditional branches, jumps, terminating traps,
/// compile-time end-borrow cleanup boundaries, descriptor-driven Buffer, Region, and target-width
/// Enum cleanup calls, and regular enum failures with explicit layout metadata. Niche-pointer
/// `Option::None` and pointer-payload `Option::Some` failures use the private pointer
/// representation after semantic lowering. Full semantic cleanup schedule integration remains
/// open.
pub fn emit_module(module: &BackendModule) -> Result<String, BackendEmitError> {
    let (_context, owned_module) = lower_module(module, usize::BITS as u8)?;
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
    let (_context, owned_module) = lower_module(module, target.pointer_width.bits())?;
    verify_module(owned_module.raw)?;
    super::object::emit_object(owned_module.raw, target, output)?;
    Ok(())
}

/// Emits a validated backend module as an owned target-specific object buffer.
pub fn emit_object_bytes(
    module: &BackendModule,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    let (_context, owned_module) = lower_module(module, target.pointer_width.bits())?;
    verify_module(owned_module.raw)?;
    super::object::emit_object_bytes(owned_module.raw, target)
}

use super::module::lower_module;
