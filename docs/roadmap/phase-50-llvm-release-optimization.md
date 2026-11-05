# Phase 50 — LLVM release optimization pipeline

## Status

Proposed. This is a separate high-priority performance task and is not part
of the stack-frame evidence implementation.

## Problem

The resolved release profile carries `OptimizationLevel::Speed` through the
shared configuration and Cranelift receives that level. The LLVM emission path
currently uses its existing target-machine defaults and does not run an LLVM IR
optimization pipeline. `ObjectOptimizationLevel::Default` is only a target
machine code-generation setting; it is not a substitute for optimizing the
LLVM IR before instruction selection.

Consequently, making LLVM the default backend does not yet provide the release
profile's intended optimization behavior for LLVM-generated objects.

## Scope

- Map Actus debug and release profiles to an explicit LLVM IR optimization
  pipeline and target-machine optimization level.
- Keep optimization policy in the LLVM backend boundary; do not expose LLVM
  pass-manager types through frontend or shared semantic modules.
- Preserve integer-only emission, deterministic lowering, ownership semantics,
  ABI contracts, stack-limit proof identity, and the existing debug output
  contract.
- Define whether optimization is enabled for object emission, executable
  emission, and in-memory test fixtures independently and document the choice.

## Acceptance evidence

- Debug and release builds use the documented LLVM optimization policy.
- Release objects demonstrate the expected optimized IR/code path without
  changing observable Actus behavior.
- Semantic, ownership, ABI, zero-float, and stack-limit tests pass at both
  profiles.
- Repeated builds remain deterministic under the same compiler, target,
  profile, lockfile, and LLVM identity.
- The final report records the LLVM version, pass pipeline, target, profile,
  commands, and representative before/after evidence.

## Explicit non-scope for the current stack-contract change

No LLVM IR pass manager, release optimization mapping, or performance
benchmarking is implemented by the stack-limit contract work. This roadmap
item must be planned and reviewed separately before those changes begin.
