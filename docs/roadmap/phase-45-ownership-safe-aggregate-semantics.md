# Phase 45 — Ownership-Safe Aggregate Semantics and Backend Parity

## Status

Planned. This phase starts only after the remaining Phase 44 backend-boundary
gates have been closed or explicitly deferred with evidence. It is limited to
aggregate ownership semantics that must remain identical in LLVM and Cranelift.

This phase applies only to the Actus compiler. It does not include Twin-e,
application behavior, benchmark tuning, or a new language syntax.

## Objective

Make aggregate field and element replacement obey the existing Actus ownership
and cleanup contracts instead of relying on raw byte-copy behavior. The same
validated semantic operation must reach both native backends through the
backend-neutral lowering boundary.

The phase resolves the remaining boundary between:

```text
semantic ownership decision
    -> aggregate assignment contract
    -> cleanup and move plan
    -> backend-neutral lowering
        -> Cranelift
        -> LLVM
```

## Architectural source of truth

The implementation must follow the existing decisions:

- [ADR-0046](../decisions/ADR-0046-struct-ownership-visibility-and-representation.md)
  for value fields, `erg` fields, `abs` fields, moves, replacement, and
  rejection of implicit aggregate copies;
- [ADR-0059](../decisions/ADR-0059-value-reuse-and-fixed-slot-selection.md)
  for explicit scalar reuse and rejection of implicit copies for cleanup-bearing
  aggregates;
- [ADR-0016](../decisions/ADR-0016-ownership-semantics.md) and
  [ADR-0027](../decisions/ADR-0027-deterministic-raii-scope-drop.md) for state
  transitions and exactly-once cleanup;
- [ADR-0031](../decisions/ADR-0031-deterministic-stack-safe-struct-return-abi.md)
  for aggregate return storage;
- [ADR-0080](../decisions/ADR-0080-backend-neutral-generic-specialization.md)
  for backend-neutral generic specialization; and
- [ADR-0081](../decisions/ADR-0081-large-aggregate-target-policy.md) for
  target-aware aggregate storage policy.

No new ownership role, implicit copy rule, aggregate ABI, or backend-specific
semantic exception may be introduced in this phase.

## Required invariants

- A cleanup-bearing aggregate is never duplicated implicitly.
- A replacement destination is cleaned according to its current ownership
  state before a new value becomes live there.
- An explicit transfer invalidates the source exactly once and moves cleanup to
  the destination.
- A borrowed `abs` value never becomes an owner and never receives cleanup.
- `ins` remains a call-scoped loan and never becomes persistent field or array
  ownership.
- Partial moves remain path-sensitive and deterministic across branch joins.
- Aggregate layout, field offsets, alignment, and target policy are unchanged.
- LLVM and Cranelift receive the same validated operation kind, type, layout,
  ownership metadata, and cleanup plan.
- Backend code never repairs an invalid semantic ownership decision.
- Hosted large-aggregate storage remains explicit and freestanding targets
  continue to reject forbidden implicit allocation.

## Gates

### Gate 45.1 — Contract inventory and semantic classification

- [ ] Inventory every aggregate assignment, field store, array element store,
      return, constructor, and aggregate copy path in the semantic model and
      both backends.
- [ ] Classify each path as value construction, explicit move, borrowed view,
      exclusive loan, replacement, or unsupported operation.
- [ ] Identify all paths that currently lower to raw memory copy and document
      whether the copy is permitted by the existing ADR contract.
- [ ] Add no new syntax or fallback behavior while completing the inventory.

### Gate 45.2 — Semantic ownership and replacement rules

- [ ] Make semantic validation decide whether aggregate replacement is legal
      before native lowering.
- [ ] Track destination cleanup and source move state for aggregate field and
      indexed-element replacement.
- [ ] Preserve partial-move paths and deterministic branch joins.
- [ ] Reject implicit copy of cleanup-bearing aggregates with stable semantic
      diagnostics.
- [ ] Add negative tests for copying `Buffer`, `Region`, `Arena`, and other
      cleanup-bearing aggregates without an explicit permitted operation.
- [ ] Add positive tests for passive value aggregates and explicit ownership
      transfers.

### Gate 45.3 — Backend-neutral aggregate assignment contract

- [ ] Extend the shared lowering contract so aggregate operations carry the
      validated semantic operation kind and complete layout metadata.
- [ ] Represent destination cleanup, source consumption, and borrowed access
      as explicit neutral instructions or cleanup-plan actions.
- [ ] Reject incomplete ownership metadata before either backend emits code.
- [ ] Keep backend-neutral modules free of Cranelift and LLVM types.
- [ ] Add shared-lowering tests for nested, generic, indexed, and field-based
      aggregate operations.

### Gate 45.4 — Cranelift implementation and parity

- [ ] Lower the neutral aggregate contract in Cranelift without adding a
      backend-only ownership rule.
- [ ] Emit exactly-once cleanup for replaced owned destinations.
- [ ] Verify explicit moves do not produce a second source cleanup.
- [ ] Verify `abs` and `ins` paths do not create ownership or cleanup.
- [ ] Add native positive, negative, early-return, branch-join, and nested
      aggregate regressions under `tests/`.

### Gate 45.5 — LLVM implementation and parity

- [ ] Lower the same neutral aggregate contract in LLVM.
- [ ] Match Cranelift's cleanup order, failure class, layout, and observable
      result for every supported aggregate operation.
- [ ] Ensure `llvm.memmove` is used only where the validated operation is a
      permitted value copy; it must not substitute for an ownership decision.
- [ ] Add LLVM native regressions under `tests/` for the same fixtures used by
      Cranelift.
- [ ] Add explicit rejection coverage for unsupported ownership-bearing
      aggregate shapes.

### Gate 45.6 — Generic and target-profile coverage

- [ ] Verify concrete and generic nested aggregate applications preserve field
      roles and cleanup metadata after specialization.
- [ ] Verify `Option`/`Result` aggregate payloads retain their existing return
      and case-merge ABI.
- [ ] Verify hosted large aggregates use only the accepted target-aware path.
- [ ] Verify freestanding, ARM32/Thumb, and Cortex-M profiles reject forbidden
      implicit aggregate allocation before backend emission.
- [ ] Verify no backend introduces hosted runtime imports for freestanding
      aggregate programs.

### Gate 45.7 — Full evidence and documentation

- [ ] Run focused semantic, shared-lowering, Cranelift, and LLVM tests.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo check --all-targets --all-features`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run the source-limit and architecture-boundary checks.
- [ ] Record exact compiler, backend, target, linker, and fixture conditions.
- [ ] Update ADR-0046 from Proposed only if every required contract has direct
      implementation and regression evidence.
- [ ] Record unresolved limitations instead of weakening tests or adding
      allow-style exceptions.

## Explicit non-goals

- No new aggregate-copy or clone syntax.
- No implicit deep copy for aggregates with cleanup obligations.
- No change to `erg`, `abs`, `dat`, or `ins` semantics.
- No change to serialized formats, Region descriptors, or public C ABI.
- No backend-specific workaround for a shared semantic failure.
- No changes to Twin-e or any external project.

## Exit criteria

Phase 45 closes only when every gate is checked with reproducible evidence,
LLVM and Cranelift agree on accepted and rejected ownership behavior, and the
remaining raw aggregate memory-copy sites are either covered by an explicit
value-copy contract or removed from ownership-bearing paths.

The phase must remain open when a test is weakened, a source workaround is
added, a backend diverges, or ownership behavior cannot be proven at semantic
analysis time.
