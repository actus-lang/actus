# `std::region` public API

```actus
import std::region;
```

## `Region[T]`

`Region[T]` is an owned, runtime-backed descriptor for a logical sequence of
fully sized `T` elements. It exposes one bounded resident window at a time;
the logical length may be larger than the resident storage. `Region[T]` is not
an `Array[T, N]`, does not contain a raw pointer, and does not perform implicit
remapping, allocation, provider I/O, or page faults during element access.

`T` may be a fixed-width primitive, a fixed-size array, a `pack`, or a sized
`struct`. Unsized and runtime-sized element types are rejected during semantic
analysis. The compiler derives `size_of[T]()` and `align_of[T]()` at open time;
the runtime validates stride, alignment, logical bounds, window bounds, and
checked byte arithmetic before the capability becomes usable.

The owner is created by `region_open` and is released by successful
`region_close` or deterministic lexical cleanup. `abs` is a read-only view,
`ins` is an exclusive call-scoped mutable loan, and `dat` transfers ownership.
All Region operations return typed `Result` values; native status codes and
runtime pointers are not part of the public API.

## `RegionError`

| Variant | Meaning |
| --- | --- |
| `InvalidDescriptor` | Layout or region metadata does not satisfy the contract. |
| `InvalidHandle` | Capability is unknown or already released. |
| `StaleGeneration` | The supplied capability generation is no longer live. |
| `OutOfWindow` | Index is outside the active resident window. |
| `OffsetOverflow` | Logical index or byte offset cannot be represented. |
| `BufferSize` | Source or destination is not the exact requested range size. |
| `CapabilityExhausted` | Runtime capability table has no free slot. |
| `GenerationExhausted` | Capability generation cannot advance safely. |
| `BackendFailure` | Selected target provider rejected the operation. |
| `WindowBusy` | A transition conflicts with unpublished mutations, a pin, or the single-threaded loan contract. |
| `BulkLimitExceeded` | A range payload exceeds the fixed 64 KiB bulk-operation limit. |

The enum is exhaustive. Callers must handle every variant with `case`; no
implicit exception or sentinel-value path represents a Region failure.

## Read and write

```actus
open verb region_read[T](
    abs region: Region[T],
    erg index: u64,
    ins destination: Buffer
) -> Result[Int, RegionError];

open verb region_write[T](
    ins region: Region[T],
    erg index: u64,
    abs source: Buffer
) -> Result[Int, RegionError];
```

Both buffers must be exactly `size_of[T]()` bytes wide. The runtime derives the
element alignment from `align_of[T]()` and rejects a descriptor whose alignment
is zero, non-power-of-two, or greater than its stride. `region_read` copies one
resident element into the destination. `region_write` replaces one
resident element and makes the region dirty until publish or cancel.

## Bounded range access

```actus
open verb region_read_range[T](
    abs region: Region[T],
    erg start_index: u64,
    erg element_count: u64,
    ins destination: Buffer
) -> Result[u64, RegionError];

open verb region_write_range[T](
    ins region: Region[T],
    erg start_index: u64,
    erg element_count: u64,
    abs source: Buffer
) -> Result[u64, RegionError];
```

The buffer must be exactly `element_count * size_of[T]()` bytes wide and the
payload must not exceed 64 KiB. The
runtime checks the index addition, byte multiplication, resident-window
containment, and buffer length before copying. A non-empty range that crosses
the resident window is rejected; callers must explicitly remap or stage a
separate operation. An empty range is a successful no-op when its start is no
greater than the logical length, including the logical end boundary.

Range operations have an all-or-nothing validation contract. `Ok(count)` means
all requested elements were copied and `count` equals `element_count`.
`Err(error)` means zero elements were copied or changed, including when the
range overflows or the buffer has the wrong size. The operations allocate no
memory and perform no I/O. If source and resident storage overlap at a native
boundary, copying follows memmove semantics.

## Lifecycle

```actus
open verb region_publish[T](ins region: Region[T]) -> Result[u64, RegionError];
open verb region_cancel[T](ins region: Region[T]) -> Result[Int, RegionError];
open verb region_close[T](ins region: Region[T]) -> Result[Int, RegionError];
```

`publish` is the bounded transaction commit. All resident bytes are validated
before any publication state changes; on success the complete resident window
becomes the new accepted snapshot, dirty state is cleared, and the generation
advances exactly once. A stale generation, generation exhaustion, or invalid
resident layout leaves the owner, generation, dirty state, and staged bytes
unchanged, so the caller may retry with the live generation or cancel.

`cancel` is the bounded transaction rollback. It restores the complete resident
window from the latest accepted snapshot, clears dirty state, and keeps the
current generation unchanged. It is safe to call after a failed publication or
as an idempotent cleanup of a clean transaction. `close` releases the
capability; successful close ends the owner's usable lifecycle.

### Ownership, views, and cleanup

Region ownership is explicit at every call site. `region_open` and
`region_remap` consume their backing `Buffer` with `dat`; the source binding
cannot be used or dropped again. Read and inspection operations use an `abs`
Region view. Mutation, publication, cancellation, and close use an exclusive
`ins` loan. Scalar indexes, counts, and window values are `erg` inputs.

The Region value carries an opaque runtime descriptor whose generation is
checked for every operation. Views are limited to the call that created them;
they cannot escape, overlap an exclusive loan, or remain valid after close or
generation change. A stale, closed, moved, or aliased value produces a typed
failure before resident bytes are accessed.

An owned Region is released exactly once. Explicit successful close releases
the live capability and leaves descriptor cleanup to lexical ownership teardown.
Failed close preserves the owner, so the caller may inspect or retry it. Scope
exit, early return, and nested `dat` transfer all use the same deterministic
cleanup contract. The native bridge receives the descriptor pointer directly,
validates it against the bounded capability table, and rejects repeated or
unknown cleanup without dereferencing an invalid pointer.

## Window inspection and remapping

```actus
open verb region_logical_length[T](abs region: Region[T]) -> Result[u64, RegionError];
open verb region_window_start[T](abs region: Region[T]) -> Result[u64, RegionError];
open verb region_window_count[T](abs region: Region[T]) -> Result[u64, RegionError];
open verb region_generation[T](abs region: Region[T]) -> Result[u64, RegionError];
open verb region_dirty[T](abs region: Region[T]) -> Result[Int, RegionError];
open verb region_pin[T](ins region: Region[T]) -> Result[Int, RegionError];
open verb region_unpin[T](ins region: Region[T]) -> Result[Int, RegionError];
open verb region_pinned[T](abs region: Region[T]) -> Result[Int, RegionError];
open verb region_remap[T](
    ins region: Region[T],
    dat backing: Buffer,
    erg window_start: u64,
    erg window_count: u64
) -> Result[u64, RegionError];
```

Inspection is read-only and allocation-free. `region_remap` consumes the new
resident buffer and replaces the current window only after validating the
window range and exact byte capacity. It advances the generation on success.
The operation is memory-only: it performs no implicit filesystem, device, or
provider I/O. A dirty or pinned resident window returns `WindowBusy`; invalid
ranges, overflow, or an incorrect buffer size leave the current descriptor and
bytes unchanged.

## Bounds, allocation, and target behavior

Single-element and range operations validate logical indexes, resident-window
containment, checked multiplication and addition, exact buffer length, and
alignment before touching bytes. Range operations are all-or-nothing: an
`Err` result means zero elements were copied or changed. Each range is limited
to 64 KiB of resident bytes; larger transfers must be explicitly chunked.

The Region core performs no hidden allocation or filesystem, device, or
network I/O on read, write, publish, cancel, inspection, pin, unpin, or close.
Opening and remapping acquire the caller-provided bounded resident storage.
Provider load, store, flush, cancel, and recovery are explicit adapter
operations described in the runtime guide.

Hosted builds use the configured hosted provider. Freestanding builds require
an explicit target provider profile; importing this facade does not create a
device backend. Target-specific behavior is selected by the provider profile,
while the public descriptor, ownership roles, typed failures, and lifecycle
remain target-neutral.
