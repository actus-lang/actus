# ADR-0002: Alpha Buffer Layout and Runtime Contract

- Status: Accepted for Alpha implementation
- Date: 2026-09-21

## Context

ADR-0001 defines an owned resource as an opaque pointer-sized handle. The
first concrete resource needs a stable layout and a small runtime contract so
that allocation, mutation, borrowing, and destruction can be lowered without
duplicating ownership logic in the backend.

## Decision

`Buffer` is represented by a pointer-sized handle to a runtime-owned buffer
header. The header contains three target-sized fields in declaration order:

```text
data:     *mut u8
length:   usize
capacity: usize
```

The header itself and its data allocation are owned by the handle. The handle
is passed by value through the native ABI. `abs` bindings alias the same handle
for read-only operations; they do not increment a runtime reference count and
never call the destructor.

The Alpha runtime provides these internal operations:

```text
actus_buffer_allocate(length: usize) -> BufferHandle
actus_buffer_drop(handle: BufferHandle)
actus_buffer_append(handle: BufferHandle, byte: u8)
```

Allocation and append must reject integer overflow and allocation failure using
the runtime's failure path. A dropped or null handle is invalid and must never
be passed to a runtime operation. The semantic analyzer remains responsible
for proving the ownership rules before any runtime call is emitted.

`actus_buffer_drop` releases the data allocation first and then the header.
It is idempotence-protected by compiler state, not by silently accepting a
second call. The backend must not emit a second drop for a binding in `Moved`
or `Dropped` state.

The header layout is native and target-sized in Alpha. A stable external C ABI
for `Buffer` is deferred until the C interoperability phase; these symbols
are internal runtime contracts for the native backend.

## Consequences

The backend can lower `Buffer` ownership without embedding allocator policy in
the semantic analyzer. The runtime owns allocation failure and overflow
behavior, while the compiler owns lifetime correctness. Cross-target layout
tests are required before exposing `Buffer` through a foreign ABI.
