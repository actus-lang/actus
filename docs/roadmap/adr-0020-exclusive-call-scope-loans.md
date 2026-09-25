# ADR-0020 Implementation Roadmap: Exclusive Call-Scope Loans

This roadmap implements the accepted `ins` role and its bounded exclusive-loan
semantics. It does not implement returned `abs` views; those belong to
ADR-0021.

## Gate 0: Contract and allocation syntax

- [x] Confirm `ins` as a parameter and call-site role, using `buffer: ins value`.
- [x] Confirm that `ins` is not a local binding role, struct-field role, or case mode.
- [x] Audit compiler, examples, tests, and documentation for legacy allocation calls.
- [x] Replace valid allocation examples with the canonical `Buffer[...]` form.
- [x] Remove the allocation intrinsic only after equivalent `Buffer[...]` behavior exists.
- [x] Add parser fixtures for valid and invalid `ins` placement.

## Gate 1: Lexer and AST

- [x] Add `TokenKind::Ins` and scan the `ins` keyword deterministically.
- [x] Add `Role::Ins` to the AST role model.
- [x] Add an optional role annotation to call arguments.
- [x] Add source spans covering the call-site role and expression.
- [x] Split parameter-role parsing from local-binding-role parsing.
- [x] Reject `ins` local declarations and `ins` struct fields in the parser.
- [x] Add lexer tests for `ins` and identifier-boundary behavior.
- [x] Add parser tests for declarations, calls, nesting, and malformed annotations.

## Gate 2: Access-state model

- [x] Add `AccessState::Suspended { loan_id }`.
- [x] Add a semantic loan record containing owner, callee, parameter, and span.
- [x] Define transitions from mutable access to suspended access.
- [x] Define restoration from suspended access to mutable access.
- [x] Reject reads, writes, moves, drops, and borrows of suspended owners.
- [x] Reject a second exclusive loan from a suspended owner.
- [x] Preserve `OwnershipState` while an `ins` loan is active.
- [x] Add transition-matrix unit tests for valid and invalid states.

## Gate 3: Call-boundary validation

- [x] Validate declared parameter role and call-site role equality.
- [x] Resolve the root binding for every `ins` argument.
- [x] Accept active `erg` and `dat` owners as loan sources.
- [x] Reject `abs`, frozen, moved, dropped, and ambiguous sources.
- [x] Detect overlapping `ins` and `abs` access to one root resource.
- [x] Detect duplicate `ins` access in one call.
- [x] Assign deterministic loan identifiers.
- [x] Restore each owner after the call exactly once.
- [x] Add stable diagnostics for every rejected call-boundary case.

## Gate 4: Loan forwarding

- [ ] Permit forwarding an active `ins` loan to a nested helper call.
- [ ] Suspend the outer binding for the duration of the nested call.
- [ ] Restore the outer function's exclusive access after the helper returns.
- [ ] Reject parallel aliases while allowing sequential helper composition.
- [ ] Test nested forwarding across one and multiple helper levels.
- [ ] Test forwarding through named and positional calls where supported.
- [ ] Verify branch snapshots and joins preserve suspended-loan state.

## Gate 5: Cleanup and native integration

- [ ] Ensure an `ins` loan does not create an ownership drop action.
- [ ] Ensure every early return, loop exit, and scope exit closes the loan.
- [ ] Validate cleanup schedules for nested loans and owner fields.
- [ ] Keep the existing native representation for `ins` parameters.
- [ ] Add native ABI tests proving no hidden wrapper or allocation is emitted.
- [ ] Add native execution tests for mutation followed by caller reuse.
- [ ] Add negative native tests for rejected aliasing programs.

## Gate 6: Completion criteria

- [ ] Run formatter, check, clippy, tests, and source-limit checks.
- [ ] Confirm no semantic rule is inferred or repaired by codegen.
- [ ] Update ADR status and implementation notes with verified behavior.
- [ ] Mark this roadmap complete only after all diagnostics and native tests pass.
