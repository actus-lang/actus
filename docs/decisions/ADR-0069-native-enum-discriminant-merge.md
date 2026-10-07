# ADR-0069: Native Enum Discriminant Stability Across Case Merges

## Status

Accepted for implementation in Phase 36.

## Context

Native `Option[T]` and `Result[T, E]` values are represented by compiler-owned
heap objects with an explicit discriminant and payload storage. A `case`
expression can inspect that discriminant and return a value. A `case` block can
also assign to an outer binding before control reaches the next statement, or
produce its value as the final expression in the block.

The native lowering previously cloned the local binding map for each case
branch, but discarded those branch-local updates at the merge block. A branch
could therefore replace an outer enum binding, release the previous enum
object, and still leave the caller using the stale pointer from before the
case. A later discriminant read then operated on released memory and could
reach the compiler-generated exhaustiveness trap (`ud2`) even though the source
value was a valid `Some`, `Ok`, or `Err` state.

This was reproduced with a compiler-only Actus fixture that uses no Twin-e
module, persistence code, or filesystem API.

The follow-up reproducer exposed a second failure at the semantic and native
boundaries. A value-producing `case` branch written as a block was accepted by
the language parser, but semantic return validation only recognized an explicit
`return` statement. Native case typing and lowering also used a source-span
heuristic that treated a semicolon as part of the statement span and therefore
discarded the final expression. The same `Result[T, E]` type was then reported
as both expected and found even though the branch had been classified as having
no value.

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

The native lowering context now distinguishes statement cases from
value-producing case expressions. Statement cases do not require a final value,
while expression cases preserve the final expression as the branch result.
Nested short-circuit expressions retain the enclosing loop targets, and cleanup
plan lookup tolerates the compiler's normalized one-byte source-span boundary
without weakening ownership or return matching.

Semantic Result-return validation now accepts either the final expression or an
explicit returned expression in a case block. Native case typing and lowering
use the final expression statement as the branch value without relying on
source-span equality. The expression is lowered once after the preceding block
statements, then passed through the existing case merge value.

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
- A value-producing block must have a final expression or an explicit return;
  statements that do not produce a value remain fallthrough statements.
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

## Follow-up verification

- `cargo check --all-targets --all-features` passed after the fix.
- The focused `result_case_block_tail_preserves_native_identity` regression
  passed.
- The Actus application suite passed with 33 tests.
- The nested short-circuit loop regression passed after loop targets were
  propagated through recursive operation lowering.
- The generic standard-I/O regression passed after statement and
  value-producing case lowering were separated.
- The full Actus suite passed, including 33 application tests, 101 array/native
  tests, 24 source-limit tests, and 25 standard-I/O tests.
- The fixed compiler was installed and Twin-e `actus test --strict` passed with
  27 tests; a strict Twin-e executable also passed native zero-float
  verification.
