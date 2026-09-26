# ADR-0024: Role-Based Generic Stream Abstractions (`Reader` and `Writer`)

- Status: Proposed
- Date: 2026-09-26
- Scope: Phase 16 standard-library stream abstractions

## Context

The initial `std::io` surface provides useful console and buffer operations,
but the first buffered reader and writer implementations are tied to stdin and
stdout. That coupling is acceptable for the first console gates, but it does
not provide a reusable abstraction for files, sockets, or in-memory cursors.
Duplicating a buffered type for every source and destination would make the
standard library larger without adding a new ownership guarantee.

Actus already has the language mechanisms needed to state the important part
of a stream contract: `ins Buffer` fills caller-owned storage without an
allocation, while `abs Buffer` reads a non-owning view without consuming it.
The missing architectural layer is a role contract that lets a buffered
adapter operate over any compatible source or destination.

## Decision proposal

Add two standard-library roles:

```actus
role Reader {
    verb read(ins buffer: Buffer) -> Result[Int, IoError];
}

role Writer {
    verb write(abs buffer: Buffer) -> Result[Int, IoError];
    verb flush() -> Result[Int, IoError];
}
```

The exact receiver syntax and generic declaration syntax remain subject to the
compiler gates below. The snippets describe the intended public contract, not
an assertion that the generic stream layer is already implemented.

Buffered adapters will use constrained generic parameters:

```actus
struct BufferedReader[Source: Reader] {
    erg source: Source,
    erg buffer: Buffer,
}

struct BufferedWriter[Target: Writer] {
    erg target: Target,
    erg buffer: Buffer,
}
```

`BufferedReader[Source: Reader]` statically dispatches refill operations to
the selected `Source`. `BufferedWriter[Target: Writer]` statically dispatches
writes and flushes to the selected `Target`. The compiler must reject an
instantiation whose type does not satisfy the declared role bound.

## Ownership and error contracts

`Reader.read` receives `ins buffer`. The buffer belongs to the caller and is
exclusively loaned for the duration of the call. The reader may fill or append
to it, but the loan cannot escape, and the call must not allocate a replacement
buffer. A successful result reports the exact number of bytes read. Failures
are represented by `Result[Int, IoError]`; C ABI status values remain private to
the runtime boundary.

`Writer.write` receives `abs buffer`. The writer observes the caller's bytes
without taking ownership or modifying the source. `flush` reports completion
through the same typed result contract. A buffered writer owns its destination
and reusable storage through `erg` bindings; it must not create hidden heap
objects during ordinary refill, write, or flush cycles.

The underlying source or destination is transferred into an adapter with an
explicit ownership operation such as `dat`, or is constructed as an owned
field. A borrowed view may not be stored beyond its permitted call or lexical
scope. Cleanup remains deterministic and follows the existing LIFO ownership
rules.

## Implementations

The intended implementations are:

- `File` implements `Reader` and `Writer` over its owned operating-system
  handle.
- `stdin` implements `Reader`.
- `stdout` and `stderr` implement `Writer`.
- `Cursor` implements `Reader` and `Writer` over an in-memory `Buffer`.

Each implementation will keep its platform bridge behind an explicit
`unsafe extern "C"` boundary. The role contract itself remains platform
neutral. Host-only handles and services must not leak into `core`, and a
freestanding target must be able to omit implementations that require an
operating system.

## Static dispatch and representation

The first implementation uses static dispatch. Concrete generic instances are
resolved during semantic analysis and lowered to direct calls. No vtable,
dynamic role object, wrapper allocation, or hidden dispatch allocation is
introduced by a `Reader` or `Writer` bound. Dynamic dispatch is outside this
decision and requires a separate architectural record if it becomes necessary.

## Implementation gates

- [ ] Define receiver and `self` rules for role contracts.
- [ ] Extend generic declarations and bounds to express `Source: Reader` and
      `Target: Writer`.
- [ ] Validate role satisfaction and reject missing or incompatible methods.
- [ ] Define generic layout and concrete-instance discovery for stream types.
- [ ] Lower statically dispatched reader and writer calls without wrappers.
- [ ] Add `File`, console, and `Cursor` role implementations.
- [ ] Add positive and negative semantic tests for role bounds and ownership.
- [ ] Add native tests for refill, write, flush, and error propagation.
- [ ] Verify zero-allocation behavior for reusable buffer loops.
- [ ] Update the `std` facades and public documentation after the contracts
      are implemented.

## Consequences

This design gives `std::io` one reusable buffering architecture while keeping
ownership explicit and the runtime boundary small. It also makes the compiler
responsible for generic role satisfaction before code generation. The cost is
that receiver semantics, generic bounds, and concrete-instance lowering must
be specified before the standard library can expose the generic adapters as a
stable public API.
