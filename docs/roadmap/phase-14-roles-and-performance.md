# Phase 14: Roles and Performance

This phase introduces Actus role contracts and separates compile-time static
performance from explicit runtime dynamic dispatch. It depends on the generic
type system in Phase 13 and must preserve `erg`, `abs`, and `dat` semantics.

Phase 14 is also the ownership-semantics verification phase. Its success is
measured by compiler-proven invariants, not by the number of new keywords.
The normative ownership model is defined in
[ADR-0016](../decisions/ADR-0016-ownership-semantics.md).

## Implementation Gates and Invariants

Every gate follows this structure:

```text
Prerequisites -> Implemented Scope -> Verification -> Invariant -> Deferral
```

The ownership gate must prove that every owner has one ownership path, every
move invalidates its source, partial moves have deterministic cleanup, every
exit resolves ownership, `abs` cannot outlive its owner, temporary freezing
does not consume an active owner, `dat` has one destination, cleanup happens
exactly once, failed operations do not create ambiguous ownership, and unsafe
operations require an explicit `unsafe` boundary.

- [x] Add the ownership/access state model: `Active`, `Moved`,
  `PartiallyMoved`, `Dropped` plus `Mutable` and `Frozen` access.
- [x] Define and test temporary `erg`/`dat` to `abs` access.
- [x] Specify the `case abs` subject lifecycle: temporary frozen access,
  read-only payload bindings, and restoration at the case boundary.
- [x] Specify the `case dat` subject lifecycle: consuming transfer, moved
  source state, payload ownership, and branch-local cleanup.
- [x] Implement semantic state isolation for each case branch so alternative
  branches cannot mutate one shared ownership state.
- [x] Validate branch joins for compatible ownership and access states across
  all reachable branches.
- [x] Track nested partial moves using precise field paths and preserve their
  deterministic remaining cleanup state.
- [x] Define guard semantics, including evaluation order, temporary `abs`
  inspection, failed-guard ownership, and move restrictions.
- [x] Lower pattern guards with conditional native branches and rollback-safe
  fallthrough to the next pattern.
- [x] Add semantic regressions for branch conflicts, join failures, nested
  moves, and relevant early exits.
- [x] Add semantic regressions for guards and failed-guard ownership.
- [x] Add an end-to-end `examples/` showcase covering `case abs`, `case dat`,
  nested partial moves, and deterministic cleanup.
- [x] Verify all return, break, continue, and error-propagation cleanup paths.
- [ ] Define task-transfer failure ownership for future `act` support.
- [ ] Keep mutable borrowing, reborrow chains, advanced lifetime polymorphism,
  and shared mutable ownership deferred.

## Role Contracts

- [x] Add `role` declarations to the lexer and parser.
- [x] Represent role contracts and role method signatures in the AST.
- [x] Validate duplicate role names, duplicate methods, and invalid signatures.
- [x] Support receiver roles (`erg self`, `abs self`, and `dat self`).
- [x] Produce stable diagnostics for invalid role declarations.

## Static Performance

- [x] Add `perform Role for Type` declarations to the grammar and AST.
- [x] Register performances and validate that every required role verb is met.
- [x] Reject incompatible receiver roles and mismatched method signatures.
- [x] Resolve role bounds such as `[T: Writer]` during type checking.
- [x] Monomorphize reachable performances deterministically.
- [x] Lower static performance calls as direct Cranelift calls.
- [x] Verify that static dispatch emits no vtable or hidden runtime metadata.

## Ownership and Contract Semantics

- [x] Preserve `erg`, `abs`, and `dat` rules across role contracts.
- [x] Keep ownership state independent from frozen access state.
- [x] Allow temporary frozen views from active `erg` and received `dat` owners.
- [x] Reject mutation through `abs` receivers and use-after-move through `dat`.
- [x] Track deterministic cleanup for owned role receivers and payloads.
- [x] Define role-bound diagnostics for borrow, move, and cleanup failures.

## Explicit Dynamic Dispatch

- [x] Reserve and parse `dynamic` as a full language keyword.
- [x] Define `abs dynamic Role` type checking and ownership boundaries.
- [x] Specify dynamic object and fat-pointer layout.
- [x] Generate and validate role vtables for dynamic performances.
- [x] Lower dynamic calls through explicit vtable dispatch.
- [ ] Define the dynamic ABI before enabling cross-unit use.

## Testing and Completion Gates

- [x] Add parser and AST tests for roles, performances, bounds, and `dynamic`.
- [x] Add positive and negative semantic tests for receiver roles and bounds.
- [x] Add monomorphization and deterministic static-dispatch golden tests.
- [x] Add dynamic object layout and vtable tests.
- [x] Add ownership, borrowing, and cleanup regression tests.
- [x] Keep source and function limits within repository policy.
- [ ] Update the manifesto, language conventions, and user documentation.
