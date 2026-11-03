# ADR-0102: Explicit zero-on-drop secret storage contract

## Status

Accepted.

## Context

Secret material can remain in a value's storage after its ownership lifetime
ends. Ordinary cleanup releases ownership, but it does not promise that the
previous bytes are overwritten. A security-sensitive type therefore needs an
explicit contract that is visible at the declaration site and enforced on
every compiler-generated cleanup path.

## Decision

Actus will support `meta zero_on_drop` on concrete `repr(C)` structs. The
attribute is a type contract, not a request to erase arbitrary memory.

The initial contract has these boundaries:

- generic structs are rejected;
- `abs` and `ins` fields are rejected because they are borrowed views, not
  owned secret storage;
- runtime-resource fields such as `Buffer`, `Region`, and `String` are
  rejected until their nested cleanup and zeroing order has a separate proof;
- the compiler knows the exact concrete layout before accepting the attribute;
- the value is zeroed only when the owner is being cleaned up; borrowing the
  value never clears the owner's storage;
- cleanup includes normal scope exit, explicit `drop`, early return, loop
  exits, and ownership-transfer paths that retain a live owner;
- the zeroing operation is emitted through a dedicated native runtime bridge
  that uses volatile byte stores and cannot be replaced with an optimizable
  ordinary store or an allocation-heavy helper;
- the compiler must emit the zeroing operation after any nested cleanup and
  before the storage becomes unreachable or is released.

The contract is fail-closed. An unsupported representation, target, backend,
or cleanup shape is a compilation error; the compiler must not silently omit
the clearing operation.

## Alternatives rejected

- Clearing every dropped value: this would change ordinary ownership semantics
  and add hidden work to unrelated types.
- A user-callable `memset` convention: callers could forget it and the
  optimizer could remove a non-volatile store.
- A runtime finalizer: cleanup timing would become implicit and would not
  preserve Actus's visible ownership model.
- Clearing only the normal scope exit: early returns, loop exits, explicit
  drops, and partial ownership paths would remain unsafe.

## Evidence required

Implementation is not complete until parser, formatter, semantic, ownership,
runtime, LLVM, and Cranelift tests prove the positive contract and reject
missing `repr(C)`, generic/resource/borrowed fields, omitted cleanup paths, and
backend paths that would omit or reorder the zeroing operation.
