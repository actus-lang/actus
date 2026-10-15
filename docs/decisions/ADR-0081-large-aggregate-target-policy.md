# ADR-0081: Target Policy for Large Aggregate Storage

- Status: Accepted
- Date: 2026-10-14
- Scope: Backend-neutral aggregate lowering and native target contracts

## Context

Native stack frames are bounded. A large aggregate cannot be placed on the
stack safely on every target, but silently moving it to a heap allocation would
violate Actus's no-hidden-allocation and zero-implicit-magic contracts.

## Decision

Aggregates at or above `LARGE_AGGREGATE_THRESHOLD` use explicit hosted runtime
storage only when the selected target is Hosted. The target-aware shared
lowering boundary validates this policy before either Cranelift or LLVM emits
code.

Freestanding targets, including Cortex-M/Thumb targets, reject a binding whose
aggregate layout reaches the threshold with a compile-time diagnostic. Neither
backend may introduce `AGGREGATE_ALLOCATE_SYMBOL` for such a target.

Small aggregates retain their existing stack-backed representation. The hosted
large-aggregate path has explicit size/alignment cleanup through the runtime
ABI; it is not a language-level implicit allocation capability exposed to
Actus source.

## Consequences

Hosted programs can use large bounded aggregates without overflowing the native
stack. Freestanding programs must use a target-appropriate explicit storage
design or remain below the threshold. Backend parity is enforced by shared
target-aware preflight and negative tests for both native backends.
