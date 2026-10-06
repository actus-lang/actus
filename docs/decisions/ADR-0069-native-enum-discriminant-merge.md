# ADR-0069: Native Enum Discriminant Stability Across Case Merges

## Status

Accepted for implementation in Phase 36.

## Context

Native `Option[T]` and `Result[T, E]` values are represented by compiler-owned
heap objects with an explicit discriminant and payload storage. A `case`
expression can inspect that discriminant and return a value. A `case` block can
also assign to an outer binding before control reaches the next statement.

The native lowering previously cloned the local binding map for each case
branch, but discarded those branch-local updates at the merge block. A branch
could therefore replace an outer enum binding, release the previous enum
object, and still leave the caller using the stale pointer from before the
case. A later discriminant read then operated on released memory and could
reach the compiler-generated exhaustiveness trap (`ud2`) even though the source
value was a valid `Some`, `Ok`, or `Err` state.

This was reproduced with a compiler-only Actus fixture that uses no Twin-e
module, persistence code, or filesystem API.

## Decision

Native case lowering carries the values of all bindings visible before the
case through the case merge block. Each merge block receives one Cranelift
block parameter per visible binding, followed by the case result parameter.
Every fallthrough branch supplies its current binding values and its case
result. After switching to the merge block, the lowering updates the current
local map to the merged binding values.

Case expression lowering uses a cloned local map. This preserves expression
isolation while still allowing branch-local state needed to produce the case
result. Statement-level case expressions use the mutable statement local map,
so assignments made in a case block are represented by the merge parameters.

The enum representation, discriminant width, payload layout, ownership roles,
return convention, and invalid-discriminant trap are unchanged. The change
only makes the existing control-flow state explicit at the merge boundary.

## Consequences

### Positive

- Valid `Option` and `Result` discriminants survive case-block assignments and
  nested generic native calls.
- Replaced enum bindings continue to use the new pointer after branch merge.
- Invalid discriminants still fail closed through the existing trap path.
- The fix is generic compiler behavior and does not depend on application
  modules, facades, persistence formats, or engine layouts.

### Constraints

- Case merges carry visible binding values, which increases the number of merge
  block parameters for functions with many live locals.
- The implementation must continue to use bounded source constructs and the
  established ownership cleanup schedule.
- Any future change to enum representation or return ABI requires a separate
  ADR and ABI regression coverage.

## Verification

The following native executable regressions cover the repaired path:

- direct and nested generic `Option[T]` and `Result[T, E]` returns;
- `Ok` and `Err` branches;
- a generic case block that replaces an outer `Result` binding after an
  `Option` match;
- expected process exit values, with no `SIGILL`/`ud2` reached.

The focused and application suites pass after the implementation. The full
compiler checks, aggregate payload coverage, nested facade coverage, and
Twin-e retest are recorded in the Phase 36 roadmap evidence. Existing semantic
exhaustiveness validation and native trap regressions preserve fail-closed
behavior for invalid enum states.
