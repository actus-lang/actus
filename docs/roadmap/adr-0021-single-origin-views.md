# ADR-0021 Implementation Roadmap: Single-Origin Views

This roadmap implements non-owning `abs` returns and caller-scope borrow
propagation. It depends on the completed `ins` access-state foundations from
ADR-0020 but does not change exclusive-loan semantics.

## Gate 0: Return contract

- [x] Define the canonical `-> abs ViewType` return syntax.
- [x] Define `abs self` as the receiver form of the single source parameter.
- [x] Define which view types are eligible for non-owning returns.
- [x] Define the distinction between owned returns and `abs` view returns.
- [x] Reject multiple-origin returns without adding explicit origin syntax.
- [x] Add stable diagnostics for missing, multiple, and unknown origins.

## Gate 1: AST and parser

- [x] Add an access-qualified return type representation.
- [x] Preserve the combined return span and the underlying type span.
- [x] Parse `-> abs Type` for verbs, role methods, and external declarations.
- [x] Reject `abs` in unsupported type positions.
- [x] Add parser fixtures for ordinary and method returns.
- [x] Add parser rejection fixtures for malformed qualified returns.

## Gate 2: Origin model

- [x] Add semantic origin metadata for expressions.
- [x] Represent `None`, one `abs` parameter, derived origin, and unknown origin.
- [x] Count `abs self` as the method's single origin parameter.
- [x] Exclude scalar parameters from origin counting.
- [x] Reject verbs with zero or multiple `abs` source parameters.
- [x] Reject expressions derived from local owned temporaries.
- [x] Reject unknown calls and expressions with multiple origins.

## Gate 3: Origin-preserving operations

- [x] Define the approved origin-preserving intrinsic and method operations.
- [x] Track direct return of the source view.
- [x] Track field and slice derivations from the source view.
- [x] Preserve the root origin through nested view derivations.
- [x] Reject operations that combine two source roots.
- [x] Add positive provenance unit tests.
- [x] Add negative provenance and temporary-owner tests.

## Gate 4: Caller-scope propagation

- [ ] Extend call signatures with the returned view origin contract.
- [ ] Transfer origin metadata from callee result to the caller expression.
- [ ] Create the returned view's borrow record in the caller scope.
- [ ] Freeze the source owner while the returned view is live.
- [ ] Support propagation through nested view-returning calls.
- [ ] Prevent source moves, drops, and exclusive loans while a view is live.
- [ ] Thaw the source only after all derived views end.
- [ ] Add scope, nested-block, and caller-return tests.

## Gate 5: Cleanup and ownership integration

- [ ] Mark returned views as non-owning bindings.
- [ ] Exclude returned views from owned drop actions.
- [ ] Emit exactly one end-borrow action for each returned view.
- [ ] Integrate view records with LIFO cleanup ordering.
- [ ] Validate early return and loop-exit cleanup for live views.
- [ ] Reject double end-borrow and stale-view cleanup states.
- [ ] Verify interaction with partial moves and case branches.

## Gate 6: Native representation

- [ ] Define the native representation of eligible view types.
- [ ] Specify pointer, length, alignment, and target-aware ABI details.
- [ ] Distinguish view values from owned `Buffer` values in layout metadata.
- [ ] Lower view-returning calls without ownership cleanup.
- [ ] Add Cranelift ABI and layout tests.
- [ ] Add native tests for zero-copy view use and source reuse after scope end.
- [ ] Add negative native tests for source destruction while a view is live.

## Gate 7: Completion criteria

- [ ] Run formatter, check, clippy, tests, and source-limit checks.
- [ ] Verify that every accepted `abs` return has one known origin.
- [ ] Verify that caller-scope borrow propagation is deterministic.
- [ ] Verify that no view receives owned cleanup.
- [ ] Update ADR implementation notes with the accepted ABI and diagnostics.
- [ ] Mark this roadmap complete only after semantic and native tests pass.
