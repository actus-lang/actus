# ADR-0037: Compiler-Backed Zero-Allocation Analysis

- Status: Proposed
- Date: 2026-09-27
- Scope: allocation effects, real-time safety diagnostics, and editor display

## Context

Embedded and real-time code often requires a proof that a control loop does
not allocate or invoke host services. A text search for `alloc`, `Buffer`, or
runtime names cannot establish that property: allocation may be hidden behind
generic calls, library verbs, error paths, or target-specific implementations.

## Decision

Zero-allocation analysis is a compiler semantic/effect capability, not an
extension-only lint. The compiler computes an allocation-effect summary for
each verb and call edge under a target profile. The LSP exposes the summary
and diagnostics; the extension renders a safety view and source annotations.

Each public callable has an effect summary with at least:

- `NoAlloc`;
- `MayAlloc`; or
- `RequiresHostService`.

The summary includes direct causes, transitive call edges, target conditions,
and source spans. An explicit strict context (for example a target profile or
future annotation) rejects any reachable `MayAlloc` or host-service effect.
The initial implementation must not silently classify unknown code as safe.

Arena placement, stack slots, fixed-size `pack` operations, and bulk arena
reset may be classified as bounded non-heap operations when the compiler has
proved their capacity and lifetime contracts. This classification is distinct
from arbitrary pointer manipulation or a host runtime call.

## LSP and extension behavior

The LSP exposes a versioned `actus/allocationEffects` request and maps strict
violations to normal compiler diagnostics with stable codes. The extension's
view shows direct and transitive causes, target profile, confidence state
(`proven`, `unknown`, or `rejected`), and navigation to every contributing
call. Colors are supplementary; text and icons remain authoritative.

The view is advisory unless the compiler strict context rejects the program.
The extension must not claim “zero allocation” merely because no known call
was found.

## Target and library requirements

Effect summaries are target-dependent. A host `std::fs` implementation may
require host services while the same public abstraction has a freestanding
implementation. Standard-library metadata must declare effects at the
runtime boundary, and FFI declarations must be conservative unless explicitly
annotated by a trusted target contract.

## Verification

- [ ] Unit tests cover direct allocation, transitive calls, generic dispatch,
      error paths, `?` unwinding, and target filtering.
- [ ] Negative tests prove unknown effects are not accepted in strict mode.
- [ ] Freestanding tests cover arena, `pack`, and MMIO-style operations.
- [ ] LSP fixtures verify effect explanations and source navigation.
- [ ] Extension tests verify stale versions and partial semantic snapshots.

## Non-goals

This ADR does not define a garbage collector, a performance benchmark, or a
proof of bounded execution time. It does not replace hardware measurement or
permit unsafe host calls merely because a view is marked strict.
