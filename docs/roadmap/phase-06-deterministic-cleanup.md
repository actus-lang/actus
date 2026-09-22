# Phase 6: Deterministic Cleanup

- [x] Implement scope cleanup planning.
- [x] Drop active owned bindings in reverse declaration order.
- [x] Skip cleanup for moved bindings.
- [x] Skip cleanup for already dropped bindings.
- [x] End borrows at their lexical scope boundary.
- [x] Implement `return` scope unwinding.
- [x] Implement `break` scope unwinding.
- [x] Implement `continue` scope unwinding.
- [x] Ensure returned owners are not dropped during unwinding.
- [x] Add LIFO destruction tests.
- [x] Add early-return cleanup tests.
- [x] Add nested-scope cleanup tests.
- [x] Add move-and-drop interaction tests.
