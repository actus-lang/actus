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

## Struct aggregates

- `open` controls whether a struct declaration crosses a module facade; fields
  do not have a separate visibility modifier.
- Unmarked fields are value fields. `erg` fields are owned mutable subresources
  and are included in the containing owner's cleanup plan.
- `abs` fields are read-only, non-owning views. Mutation of the field or any
  nested path through it is rejected.
- `ins` fields are rejected because an exclusive loan is call-scoped and cannot
  become persistent aggregate state. `dat` is an operation-level transfer and
  is not a field declaration role.
- Structs are not implicitly copyable. Compatible whole-struct assignment
  transfers the source owner, cleans the destination's previous owned value,
  and reports use-after-move on later source access.
- Partial field moves are tracked by field path and cleanup skips moved fields
  exactly once. Self-assignment and overlapping moves are rejected.
- Ordinary struct layout is deterministic by declaration order and target
  alignment. Packed and public C-ABI representations require their explicit
  contracts; unsupported aggregate ABI exposure is rejected or deferred.

## Numeric Comparisons and Output

The relational operators `<`, `<=`, `>`, and `>=` return `Bool`. Signed integer
operands use signed comparison semantics; unsigned integer operands, including
`Usize`, use unsigned comparison semantics; and `f32` and `f64` use their
corresponding floating-point comparison semantics. The semantic analyzer rejects
mixed numeric families instead of inserting an implicit conversion.

When `print` receives an `abs Buffer`, the runtime writes exactly the Buffer's
current length. The operation is binary-safe and does not read beyond the live
range or require a null terminator.

## Operators, casts, and bounded storage

Equality, remainder, logical, bitwise, shift, and relational operators are
validated according to their operand families. `&&` and `||` are genuine
short-circuit control flow: the right operand is not evaluated when its value
cannot affect the result. Division and remainder by zero, invalid shift
counts, and out-of-bounds indexing have deterministic native failure paths.

`expr as Type` is the only supported primitive integer conversion syntax.
Constant casts are range-checked during semantic analysis; dynamic casts use
native overflow checks. `Array[T, N]` storage is contiguous and bounded, while
`Buffer` indexing is checked against its live length. `ins` loans mutate the
caller-owned storage in place and restore the caller's ownership state after
the call.
