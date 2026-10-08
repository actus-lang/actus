# `std::region` runtime and target boundary

The public region API is provider-neutral. A runtime provider may back the
region with in-memory storage, mapped pages, a device, or another bounded
backend, but `Region[T]` source code sees only its checked descriptor contract.

## Provider obligations

A compatible provider must preserve:

- fixed layout validation for `T`;
- logical length versus resident window distinction;
- capability and generation checks;
- checked read, write, publish, cancel, and close behavior;
- deterministic cleanup without leaked handles;
- typed errors for capacity, stale state, bounds, and lifecycle failures.

The provider must not expose raw OS pointers in a region descriptor or make
filesystem access implicit in a read/write operation.

## Hosted and freestanding use

Hosted builds select a configured provider. Freestanding builds need an
explicit target provider; importing the facade alone does not create storage
or prove that a device backend exists.

## Scaling boundary

Logical capacity can exceed resident memory. The compiler/runtime may keep
layout metadata symbolic and use bounded windows, but the public contract does
not promise that every logical element is simultaneously resident. A target
adapter must document its window size, backing storage, publish/cancel cost,
and failure behavior.

## Provider contract

The runtime provider boundary uses a fixed `RegionProviderRequest` containing the
capability handle, generation, resident window, exact byte length, and the
current `pinned` state. A
provider implements five explicit operations:

- `load` copies the last accepted bytes into caller-owned resident storage;
- `store` copies caller-owned bytes into bounded provider staging storage;
- `flush` accepts staged bytes as the provider generation;
- `cancel` restores staging from the last accepted generation;
- `recover` loads the last accepted generation after an interrupted operation.

Provider operations return bytes completed or one of the explicit provider
failures: unavailable, cancelled, timed out, corrupt, truncated, rejected,
retryable, invalid request, or buffer too small. The core maps provider failure
classes to the public `BackendFailure` domain without exposing platform status
codes.

The hosted `MemoryRegionProvider` is a fixed-capacity, memory-only reference
provider. It owns one bounded accepted snapshot and one bounded staging buffer;
each operation receives caller-owned buffers and performs no filesystem,
device, or network I/O. Its deterministic failure injection is test-only
evidence for preserving the accepted snapshot after a provider error.

When `pinned` is true, `load` and `recover` return the typed `WindowBusy`
condition and cannot replace resident bytes. Staging, flush, cancel, and
integrity validation remain available because they do not evict the window.

Provider instances and any provider queue remain outside the public `Region[T]`
descriptor. The initial contract has no implicit asynchronous queue: queued work,
if introduced by a later adapter, must declare its capacity and ownership
explicitly. The core queue capacity is zero; a range operation is bounded to
64 KiB and returns `BulkLimitExceeded` above that limit.

## Integrity and recovery

The reference provider records a CRC32 checksum for both the accepted snapshot
and the staged bytes. A load or recovery operation validates the accepted
snapshot before copying it. A flush validates the accepted snapshot and staged
bytes before replacing the accepted snapshot, so a checksum mismatch or
truncated staging buffer cannot publish partial state.

The provider tracks the accepted generation separately from the staged
generation. A failed flush leaves the previous complete generation readable;
the caller may retry the same staged operation or cancel it. Requests for an
older generation return `StaleGeneration`, and generation overflow returns
`GenerationExhausted` without changing provider state.

A provider may enter `ReadOnly` degradation, which permits reads and recovery
but rejects mutation, or `Terminal`, which rejects all operations. These states
are explicit results and do not silently convert failed writes into successful
publication. CRC32 provides corruption detection and continuity checks; it is
not an authentication or encryption mechanism.

## ABI and hot path

Native lowering may pass the small descriptor directly or use an indirect ABI
representation. Actus source must not depend on that choice. Region access is
explicit and must not add hidden filesystem I/O, allocation, or page faults to
a caller's real-time path.
