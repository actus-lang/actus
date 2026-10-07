# ADR-0077: Freestanding Region Provider Core

- **Status:** Proposed
- **Date:** 2026-10-07
- **Scope:** No-host capability storage, resident windows, cleanup, and target
  adapters for `std::region`

## Context

The Actus compiler can now emit freestanding objects for the complete
`std::region` operation surface. Those objects intentionally import a small
provider ABI instead of linking the hosted Rust runtime. The missing piece is
the target-neutral implementation behind that ABI.

The provider must run without libc, filesystem access, a general allocator, or
operating-system pointers. It must also preserve the public `Region[T]`
contract: checked windows, generation validation, explicit publication and
cancellation, and deterministic cleanup.

`region_cancel` introduces a storage requirement that cannot be hidden. A
provider must retain the last published bytes while allowing the current
resident window to become dirty. A single mutable byte range cannot provide
both states without changing cancellation semantics.

## Decision

Create a separate target-neutral freestanding provider core. Keep it outside
compiler lowering and outside the hosted `src/runtime/region_bridge.rs`
implementation. Target adapters will supply bounded storage and expose the
stable C ABI symbols required by the generated object.

### Capability handle

The provider uses a `u64` target-agnostic handle:

- low 32 bits: slot index plus one; zero is invalid;
- high 32 bits: monotonic slot generation; zero is invalid;
- slot reuse increments the generation;
- generation overflow rejects allocation;
- a handle with a stale generation is rejected before storage access.

The handle is an index and generation value, never a raw pointer. It may be
serialized as fixed-width little-endian metadata without becoming process
specific.

### Bounded storage

Each active capability owns two target-provided byte windows of equal length:

1. **Resident window:** the bytes used by read and write operations.
2. **Published mirror:** the last successfully published bytes used by cancel.

The provider owns the resident backing transfer received by
`actus_region_open`. It must acquire or reserve the published mirror from a
bounded target pool before publishing the capability. If the pool has no
matching capacity, open returns a typed failure and leaves the caller's
ownership on the failure path.

The core never grows either window, maps pages, calls a filesystem, or silently
allocates. A target adapter may place the pair in SRAM, external memory, or a
board-specific fixed pool, but the storage budget must be explicit in its
configuration and evidence.

The target adapter must define a bounded maximum window size. `publish` and
`cancel` copy the resident window and published mirror in full, so their
latency is deterministic but proportional to the configured window. The pool
must align both windows to the element alignment required by the target; an
adapter must reject a pool entry that cannot satisfy that alignment rather than
allowing an unaligned hardware access.

### Lifecycle

1. `open` validates element stride, logical length, window bounds, address
   profile, and exact resident capacity.
2. The provider allocates a free capability slot and reserves the published
   mirror. It copies the initial resident bytes into the published mirror.
3. `read` and `write` validate handle, generation, logical index, window
   membership, and exact byte width before touching either window.
4. `publish` copies resident bytes into the published mirror, clears dirty
   state, and advances generation monotonically.
5. `cancel` copies the published mirror back into the resident window and
   clears dirty state without changing generation.
6. `close` releases the capability and both storage reservations. Repeated or
   stale close operations return a typed failure.
7. Compiler-generated cleanup calls `actus_region_drop`. Drop is idempotent at
   the provider boundary and never releases an unrelated reused slot.

### ABI boundary

The target adapter must provide these symbols with the signatures emitted by
the standard facade:

```text
actus_region_open
actus_region_read
actus_region_write
actus_region_publish
actus_region_cancel
actus_region_close
actus_region_drop
actus_buffer_drop
actus_enum_drop
```

The provider returns the existing typed `RegionError` domain through the Actus
Result ABI. It must not introduce a second error encoding or expose a target
pointer in the `Region[T]` value.

## Implementation gates

- Define the provider core as a separate no-host package with no standard
  allocator or operating-system dependency.
- Implement fixed capability slots, generation checks, paired window storage,
  dirty tracking, publication, cancellation, and cleanup.
- Add a target adapter that binds the core to a statically bounded storage pool.
- Add object-link tests proving all nine provider symbols resolve without the
  hosted runtime archive.
- Add provider unit tests for stale handles, generation exhaustion, capacity
  exhaustion, exact buffer sizes, publication, cancellation, and repeated
  cleanup.
- Publish target-specific resident-memory and execution evidence.

## Consequences

The public one-buffer `region_open` API remains stable. The target adapter
provides the second published mirror internally from a bounded pool, so
cancellation retains its existing semantics without hidden heap allocation.
Targets with insufficient paired storage fail deterministically at open. A
freestanding object can be generated before a target adapter exists, but a
freestanding executable cannot link or run until all nine symbols are supplied.

The provider core is a runtime boundary, not compiler lowering. It must not be
implemented by adding hosted services to the compiler or by weakening the
freestanding import contract.

## Current evidence

The standalone provider core currently builds as a `no_std` library and has
unit coverage for resident/published lifecycle behavior, publication and
cancellation, stale generation rejection, exact window sizing, and bounded
slot capacity. Target adapter implementation, ABI linking, generation
exhaustion evidence, and hardware measurements remain open implementation
gates.
