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

### Adapter ownership handoff

The global C ABI adapter is a separate layer above this core. On
`actus_region_open`, it must validate the incoming `ActusBuffer`, copy or
adopt its bytes into a reserved bounded resident window, reserve the paired
published mirror, and complete the `dat` ownership handoff before returning a
successful Result. The adapter must call the target's buffer release bridge
exactly once after the handoff succeeds, and must release or restore ownership
on every failure path.

The adapter must retain enough target-owned state to keep both windows live
until close or drop. A wrapper that merely forwards a borrowed byte slice while
leaving the consumed Buffer handle unmanaged is rejected because it leaks the
ownership contract and cannot provide deterministic cleanup.

## Implementation gates

- Define the provider core as a separate no-host package with no standard
  allocator or operating-system dependency.
- Implement fixed capability slots, generation checks, paired window storage,
  dirty tracking, publication, cancellation, and cleanup.
- Add a target adapter that binds the core to a statically bounded storage pool.
- Define and test the `Buffer` ownership handoff and failure cleanup before
  exporting global `actus_region_*` symbols.
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
cancellation, stale generation rejection, exact window sizing, bounded slot
capacity, and the eight-byte-aligned fixed window pool. A freestanding
`x86_64-unknown-none` build was attempted but
the target is not installed in the current toolchain (`can't find crate for
core`); this is environment evidence, not target execution evidence. Target
adapter implementation, ABI linking, generation-exhaustion evidence, and
hardware measurements remain open implementation gates.

The provider also now exposes a checked ABI contract module for the existing
Actus buffer layout, result sizes and payload offsets, descriptor size, and
public error-code mapping. This module defines the contract only; it does not
allocate result objects or claim to provide a target adapter.

The adapter boundary now has a target-neutral ownership handoff helper. Its
tests prove that a valid open calls the release callback once after provider
acceptance, while validation or provider failure leaves the incoming Buffer
owned by its caller.

The provider unit suite also forces a slot generation to `u32::MAX` in a
test-only path and verifies that reuse returns `GenerationExhausted` without
publishing a capability.

A feature-gated fixed-storage target ABI shim now exposes the seven
`actus_region_*` symbols with eight capability slots and 1024-byte resident
windows. `cargo check --features target-abi` proves that the shim compiles as
`no_std`; linking it requires target-provided `actus_buffer_drop` and
`actus_enum_allocate` symbols and remains a separate target gate.

A host integration fixture supplies those two symbols as test doubles and
executes open, write, publish, read, cancel, close, and drop successfully. The
fixture proves the ABI link and lifecycle contract only; it is not evidence of
MCU memory placement, interrupt safety, or target timing.

The shim also builds for the installed `thumbv7em-none-eabihf` target and
produces an ARM EABI relocatable object exporting all seven Region symbols.
This confirms target compilation and symbol emission for a Cortex-M4F-class
profile. The target's allocator link, resident-memory report, and cycle-level
execution measurement remain open.

A build-only fixed no-heap target fixture was linked into the ARM object. It
defines `actus_buffer_drop`, `actus_enum_allocate`, and `actus_enum_drop`; the
result exports all ten expected support/Region symbols and has no unresolved
`actus_*` symbols. This is link evidence only. A board-specific allocator and
buffer pool still must replace the fixture before hardware acceptance.
