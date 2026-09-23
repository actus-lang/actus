# Phase 14: Roles and Performance

This phase introduces Actus role contracts and separates compile-time static
performance from explicit runtime dynamic dispatch. It depends on the generic
type system in Phase 13 and must preserve `erg`, `abs`, and `dat` semantics.

## Role Contracts

- [ ] Add `role` declarations to the lexer and parser.
- [ ] Represent role contracts and role method signatures in the AST.
- [ ] Validate duplicate role names, duplicate methods, and invalid signatures.
- [ ] Support receiver roles (`erg self`, `abs self`, and `dat self`).
- [ ] Produce stable diagnostics for invalid role declarations.

## Static Performance

- [ ] Add `perform Role for Type` declarations to the grammar and AST.
- [ ] Register performances and validate that every required role verb is met.
- [ ] Reject incompatible receiver roles and mismatched method signatures.
- [ ] Resolve role bounds such as `[T: Writer]` during type checking.
- [ ] Monomorphize reachable performances deterministically.
- [ ] Lower static performance calls as direct Cranelift calls.
- [ ] Verify that static dispatch emits no vtable or hidden runtime metadata.

## Ownership and Contract Semantics

- [ ] Preserve `erg`, `abs`, and `dat` rules across role contracts.
- [ ] Reject mutation through `abs` receivers and use-after-move through `dat`.
- [ ] Track deterministic cleanup for owned role receivers and payloads.
- [ ] Define role-bound diagnostics for borrow, move, and cleanup failures.

## Explicit Dynamic Dispatch

- [ ] Reserve and parse `dynamic` as a full language keyword.
- [ ] Define `abs dynamic Role` type checking and ownership boundaries.
- [ ] Specify dynamic object and fat-pointer layout.
- [ ] Generate and validate role vtables for dynamic performances.
- [ ] Lower dynamic calls through explicit vtable dispatch.
- [ ] Define the dynamic ABI before enabling cross-unit use.

## Testing and Completion Gates

- [ ] Add parser and AST tests for roles, performances, bounds, and `dynamic`.
- [ ] Add positive and negative semantic tests for receiver roles and bounds.
- [ ] Add monomorphization and deterministic static-dispatch golden tests.
- [ ] Add dynamic object layout and vtable tests.
- [ ] Add ownership, borrowing, and cleanup regression tests.
- [ ] Keep source and function limits within repository policy.
- [ ] Update the manifesto, language conventions, and user documentation.
