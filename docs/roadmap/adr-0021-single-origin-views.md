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

- [x] Extend call signatures with the returned view origin contract.
- [x] Transfer origin metadata from callee result to the caller expression.
- [x] Create the returned view's borrow record in the caller scope.
- [x] Freeze the source owner while the returned view is live.
- [x] Support propagation through nested view-returning calls.
- [x] Prevent source moves, drops, and exclusive loans while a view is live.
- [x] Thaw the source only after all derived views end.
- [x] Add scope, nested-block, and caller-return tests.

## Gate 5: Cleanup and ownership integration

- [x] Mark returned views as non-owning bindings.
- [x] Exclude returned views from owned drop actions.
- [x] Emit exactly one end-borrow action for each returned view.
- [x] Integrate view records with LIFO cleanup ordering.
- [x] Validate early return and loop-exit cleanup for live views.
- [x] Reject double end-borrow and stale-view cleanup states.
- [x] Verify interaction with partial moves and case branches.

## Gate 6: Native representation

- [x] Define the native representation of eligible `abs Buffer` views as a
  `BufferHandle` pointer to the runtime data/length/capacity record.
- [x] Specify pointer, length, alignment, and target-aware ABI details through
  the existing `ActusBuffer` C layout and target pointer width.
- [x] Distinguish view values from owned `Buffer` values in semantic cleanup
  metadata; both use the same zero-copy native pointer representation.
- [x] Lower view-returning calls without ownership cleanup.
- [x] Add Cranelift ABI and layout coverage through the existing Buffer ABI
  checks and native execution test.
- [x] Add native tests for zero-copy view use and source reuse after scope end.
- [x] Reject source destruction while a view is live before native emission;
  negative semantic tests cover this native gate.

## Gate 7: Completion criteria

- [x] Run formatter, check, clippy, tests, and source-limit checks.
- [x] Verify that every accepted `abs` return has one known origin.
- [x] Verify that caller-scope borrow propagation is deterministic.
- [x] Verify that no view receives owned cleanup.
- [x] Update ADR implementation notes with the accepted ABI and diagnostics.
- [x] Mark this roadmap complete only after semantic and native tests pass.
