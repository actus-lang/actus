# Actus Alpha Guarantees

This document records the guarantees implemented by the Alpha compiler.

## Ownership

- `erg` is an owned binding and may be read, mutated, borrowed, moved, or
  dropped.
- `dat` transfers ownership. The caller binding becomes `Moved`, and the
  receiving function becomes responsible for cleanup.
- `ins` creates an exclusive call-scope loan. The source owner becomes
  `Suspended` during the call and returns to mutable access afterward.
- Returning a value transfers ownership to the caller.
- Returning `abs` is allowed only for a single-origin non-owning view.
- Explicit `drop` changes an owned binding to `Dropped`; cleanup never drops
  the same binding twice.

## Borrowing

- `abs name = ref owner` creates a lexical, read-only borrow.
- Active borrows are represented by `BorrowRecord` entries containing the
  owner, scope, identity, and source span.
- An owner with an active borrow is derived as `Frozen`.
- Frozen owners cannot be mutated, moved, or dropped.
- Suspended owners cannot be read, mutated, moved, dropped, or borrowed.
- `ins` may be forwarded to nested helper calls, but never aliased in one
  call.
- Returned views create caller-scope borrow records and cannot be stored in a
  longer-lived structure.
- Borrowed bindings end automatically at their lexical scope boundary.

## Cleanup

Owned bindings are cleaned up in reverse declaration order at scope exit.
Early exits unwind the affected scopes before transferring control. Moved and
already-dropped bindings are excluded from cleanup.

These guarantees are enforced before native code generation. They do not
depend on the native backend or on a garbage collector.
