# ADR-0079: Complete Runtime-Backed Logical Regions

- **Status:** Proposed
- **Date:** 2026-10-08
- **Scope:** `std::region`, compiler lowering, runtime provider contracts, bounded logical storage, and region acceptance evidence
- **Decision owner:** Actus standard-library and compiler architecture

## 1. Decision summary

Actus will develop `std::region` as a complete, target-neutral abstraction for
large logical storage with bounded resident memory. A region is an owned,
capability-checked resource that exposes a logical index space while keeping
resident storage, access windows, lifecycle state, and provider behavior
explicit.

`Region[T]` is a distinct type. It is never an implicit form of
`Array[T, N]`, a hidden heap, a raw pointer, a filesystem handle, or an
unbounded collection. The public contract must remain valid on hosted and
freestanding targets and must not require a particular operating system,
virtual-memory facility, allocator, or device.

The implementation will be completed only when every public operation has a
precise ownership contract, deterministic failure behavior, native ABI
coverage, provider-neutral semantics, and reproducible evidence.

## 2. Problem statement

The current Region foundation supports a logical length, a fixed resident
window, checked element access, publication, cancellation, and close. The
foundation does not yet define the complete lifecycle required by applications
that must move through many logical windows while retaining bounded resident
memory.

The missing design must answer all of these questions explicitly:

- How is a resident window replaced or remapped?
- How are dirty bytes published before eviction?
- How are stale handles, generations, views, and leases rejected?
- How can a caller read or write a bounded range without hidden allocation?
- How does a provider report capacity, I/O, corruption, retry, and recovery?
- How are alignment, stride, overflow, and target ABI rules enforced?
- What happens when a window is pinned, dirty, busy, or unavailable?
- How are concurrent readers and one exclusive writer represented?
- How are interrupted publication and previous-generation recovery handled?
- Which operations are memory-only and which explicitly cross a provider
  boundary?

The answers belong in the language and standard-library contract before
application code relies on them.

## 2.1 Phase 38 compatibility baseline

Phase 38 established the current compatibility baseline. The following
behavior is already part of the accepted Region foundation and must remain
source-compatible unless a later ADR explicitly changes it:

- `Region[T]` is a distinct runtime-backed type and is not an inline array.
- `T` must be a fully sized primitive, array, pack, or struct accepted by the
  compiler's layout rules.
- `region_open` consumes caller-owned `Buffer` storage with `dat` and records
  logical length, window start, window count, and element stride.
- `region_read` uses an `abs` Region and an `ins` exact-size destination.
- `region_write` uses an `ins` Region and an `abs` exact-size source.
- `region_publish` advances the accepted generation only after successful
  publication.
- `region_cancel` restores the most recently published resident bytes.
- `region_close` releases the capability exactly once; failed close preserves
  the owner according to the existing ownership contract.
- Invalid descriptors, out-of-window indexes, overflowed offsets, wrong buffer
  sizes, stale generations, exhausted capabilities, and backend failures are
  typed `RegionError` results.
- The current API performs no implicit filesystem or device I/O on open, read,
  write, publish, cancel, or close.
- Native lowering preserves Region descriptor identity through direct calls,
  nested generic calls, aggregate returns, and deterministic cleanup.
- Phase 38 evidence covers primitive and packed elements, bounded resident
  windows, logical capacities through TiB-scale profiles, and the documented
  zero-floating-point package contract.

The Phase 38 baseline does not yet provide window remapping, bounded range
operations, provider-neutral paging, pinning, or complete recovery semantics.
Those are Phase 41 work and must not be described as existing behavior.

## 3. Architectural principles

### 3.1 Distinct storage models

- `Array[T, N]` is contiguous inline storage with a fixed compile-time extent.
- `Region[T]` is logical storage with a bounded resident representation.
- A region may expose only the resident window selected by its descriptor.
- No implicit conversion exists between an array and a region.
- Logical capacity is not a promise of resident RAM, executable image size, or
  provider backing size.

### 3.2 Explicit movement

Window movement is an explicit operation. A read or write never causes an
implicit page fault, filesystem operation, provider call, allocation, flush,
or eviction. The caller chooses when a window is opened, remapped, published,
cancelled, or closed.

### 3.3 Bounded storage

Every region operation has a declared bound for resident bytes, metadata,
capability slots, queued work, and provider-visible payloads. A larger logical
length must not cause one object per logical element to appear in compiler AST,
IR, object files, or resident memory.

### 3.4 Ownership and capability safety

A region descriptor is a target-agnostic capability handle with a monotonic
generation. Public descriptors contain fixed-width values only; they contain no
raw operating-system pointer and no provider-specific type. A successful close
consumes the capability exactly once. A failed operation preserves the owner
when the contract says recovery is possible.

### 3.5 Determinism

For the same descriptor, generation, provider result, and input bytes, the
observable result and state transition are deterministic. Provider latency may
vary, but it must not change ownership, bounds, generation, or publication
semantics.

### 3.6 Separation of concerns

The standard library defines typed contracts and bounded state transitions.
Providers implement an explicit backend boundary. The compiler lowers the
public operations without inferring provider behavior. Filesystem, device, or
operating-system adapters remain outside the core Region API.

## 4. Public capability model

The public `Region[T]` descriptor must represent, at minimum:

- a target-agnostic capability handle;
- a storage generation;
- logical length;
- resident window start and count;
- element stride and alignment contract;
- dirty, pinned, and lifecycle state;
- provider or storage profile identity where required by validation.

The exact native representation is an implementation contract and may use a
small aggregate or multi-register ABI. It must not expose a runtime pointer.
The public type must remain opaque to callers while preserving stable generic
identity through nested calls, returns, facade imports, and cleanup lowering.

### 4.1 Current Gate 41.3 identity and layout profile

The current compatible descriptor profile fixes the following widths without
changing the Phase 38 public Region operations:

- `RegionHandle` is `u64`.
- The low 32 bits encode a one-based capability slot index.
- The high 32 bits encode the slot generation.
- `RegionGeneration` is `u64` and starts at `1`; zero is never authorized.
- `element_stride`, `element_alignment`, `logical_length`, `window_start`, and
  `window_count` are `u64` values.
- `dirty`, `logical_index_bits`, and `byte_offset_bits` are `u8` values.
- `element_alignment` must be non-zero, a power of two, and no greater than
  `element_stride`.
- The runtime `repr(C)` descriptor is 64 bytes, aligned to 8 bytes, with a
  descriptor ABI version of `2`. Its fixed field offsets are: handle `0`,
  stride `8`, alignment `16`, logical length `24`, window start `32`, window
  count `40`, generation `48`, dirty `56`, logical-index width `57`, and
  byte-offset width `58`.

The descriptor ABI version is checked at the runtime boundary. A version-1
descriptor is not silently reinterpreted as version 2; mixed compiler/runtime
artifacts must fail with the existing typed descriptor compatibility error.

The compiler exposes `size_of[T]()` and `align_of[T]()` as compile-time layout
intrinsics. Region opening derives both values from the fully sized element
type. The runtime validates multiplication, addition, logical extent, window
range, and byte offsets with checked arithmetic before native access.

Capability allocation increments the slot generation before returning a
handle. Releasing a slot never resets that generation. Reusing the slot
therefore produces a different handle, and the old handle is rejected even
when its numeric slot index is reused. Generation exhaustion is a typed
failure; it is never allowed to wrap to an earlier identity.

The canonical handle encoder and decoder are shared by the capability table
and hosted Region bridge. The public descriptor contains only fixed-width
integers; pointers used internally by the hosted bridge remain outside the
Actus value and outside the public facade.

## 5. Required operation families

The final Region library must define and document operation families equivalent
to the following. Names and signatures require implementation-time review, but
no family may be omitted without an ADR update.

### 5.1 Creation and inspection

- Open a logical region from caller-owned resident storage.
- Validate element size, alignment, logical length, window bounds, and
  provider capacity.
- Inspect logical length, window range, stride, generation, dirty state, and
  pin state through read-only views.
- Return typed errors without leaking native status values.

### 5.2 Window management

- Select or remap a bounded resident window explicitly.
- Reject remapping while an incompatible mutable loan, pin, or publication is
  active.
- Define whether remapping is immediate, staged, or provider-mediated.
- Preserve the old generation until the transition is accepted.
- Report unavailable, dirty, pinned, stale, or provider-rejected transitions.

Gate 41.4 implements the memory-only portion of this family. Inspection verbs
return logical length, current window start and count, generation, and dirty
state without changing ownership. `region_remap` consumes caller-owned
resident storage, validates its exact byte capacity and logical range, rejects
dirty state with `WindowBusy`, and advances generation only after the new
window is accepted. No provider call, filesystem operation, implicit page
fault, or eviction is performed. Persistent pinning and provider-mediated
window admission remain later provider and concurrency contracts.

### 5.3 Element and range access

- Read and write one fully sized element with checked logical indexing.
- Read and write bounded ranges using caller-owned buffers.
- Define overlap behavior for range copies.
- Define partial progress and failure reporting for bounded bulk operations.
- Prohibit access outside the resident window and prohibit arithmetic overflow.

Gate 41.5 defines `region_read_range` and `region_write_range` with caller-owned
buffers and an explicit `u64` element count. The entire logical range, resident
window, byte extent, and buffer length are validated before the first byte is
copied. A successful operation returns the requested element count; a failed
operation returns a typed `RegionError` and guarantees zero element progress.
This is the partial-progress contract: range operations are atomic with
respect to validation, so callers never have to infer how many elements were
mutated from an error. Copies use memmove semantics if an implementation ever
receives overlapping storage. Empty ranges are valid no-ops when their start
position is within the logical extent; non-empty ranges may not cross the
resident window. The implementation performs no allocation or I/O.

### 5.4 Publication and transactions

- Publication is an explicit bounded transaction commit. The resident storage
  is the transaction's staging area and a same-sized published snapshot is the
  last accepted state. Range validation happens before any bytes are changed;
  a successful `publish` validates the descriptor and both bounded storage
  lengths, copies the complete resident window into the snapshot, clears the
  dirty bit, and advances the generation exactly once.
- `cancel` is the rollback operation. It copies the complete accepted snapshot
  back into resident storage, clears the dirty bit, and keeps the current
  generation. It is valid for dirty and clean state, so cleanup can be
  deterministic and idempotent.
- A stale generation, generation exhaustion, invalid layout, or other failed
  publication is rejected before the snapshot or generation changes. Dirty
  staged bytes remain available for retry or cancellation, and the last
  accepted snapshot remains intact.
- A multi-element update is visible as one bounded transaction: all requested
  elements are staged in resident storage, and readers observe them as an
  accepted set only after `publish` succeeds. The core performs no filesystem,
  provider, or implicit page-fault I/O during this transition.
- The publication order is: validate live capability and generation; validate
  bounded resident and snapshot extents; copy the complete resident window;
  clear dirty state; then advance generation. No partially validated bytes may
  become the accepted snapshot.

### 5.5 Lifetime and cleanup

- Close a region explicitly and release the capability exactly once.
- Drop an owned region deterministically at scope exit.
- Preserve ownership after recoverable failure.
- Reject use after close, stale generation, invalid handle, and double release.
- Ensure explicit close does not cause a second compiler-generated cleanup call.

#### 5.5.1 Accepted ownership and native cleanup contract

Every public Region operation states its Actus role at the declaration and at
each call site. Opening and remapping consume resident buffers with `dat`.
Read-only inspection and reads use `abs` Region views; writes, publication,
cancellation, and close use an exclusive `ins` Region loan. Indexes, counts,
and layout values are owned scalar inputs (`erg`). A `dat` transfer is written
explicitly by the caller and the source binding is unavailable after the call.

The descriptor stored in an Actus `Region[T]` value is an opaque pointer to the
runtime-owned descriptor. Its generation is copied into every hosted view and
is checked before the view reaches resident storage. Views are call-scoped;
their implementation uses Rust lifetimes, so a view cannot escape the
operation or coexist with a conflicting remap, close, or publication.

The native cleanup bridge accepts the descriptor pointer directly. It searches
the bounded descriptor table without dereferencing an unknown pointer, removes
the matching resident capability, and drops the descriptor exactly once. A
closed Region retains its descriptor registration until lexical cleanup so an
explicit successful close and automatic cleanup are complementary operations,
not two releases of the same descriptor. Unknown and repeated cleanup returns
failure without dereferencing the supplied pointer.

Recoverable close and lifecycle failures preserve the owner and its capability.
The runtime validates descriptor identity and generation before taking a slot;
therefore a failed close, stale view, or invalid access cannot consume the
owner. Early returns, lexical scope exits, nested ownership transfer, and
explicit close are all covered by native execution evidence. A transferred
`dat` value is cleaned by its receiving scope, while the source receives no
second cleanup action.

### 5.6 Provider boundary

- The runtime provider boundary is target-neutral and receives a fixed
  `RegionProviderRequest` containing the capability handle, generation, resident
  window, and exact byte length. It defines explicit `load`, `store`, `flush`,
  `cancel`, and `recover` operations. The provider returns a bounded byte count
  or a typed provider failure; it does not return platform status codes.
- `load` and `recover` write only to caller-owned destination storage. `store`
  reads caller-owned source storage. `flush` accepts provider staging and
  `cancel` restores staging from the last accepted provider generation. Provider
  buffer ownership therefore remains explicit at every boundary.
- The provider failure domain distinguishes invalid request, buffer too small,
  unavailable, cancelled, timed out, corrupt, truncated, rejected, and
  retryable outcomes. The runtime maps these classes to the public
  `BackendFailure` family without pretending that a failed provider operation
  succeeded.
- The hosted `MemoryRegionProvider` is the deterministic reference provider. It
  has one fixed-capacity accepted snapshot and one fixed-capacity staging buffer,
  performs no external I/O, and supports deterministic one-shot failure
  injection for tests. It proves staging, flush, cancel, recovery, capacity,
  and failure-preservation behavior without selecting a platform.
- Provider-specific types, queues, OS handles, and raw pointers remain outside
  the public `Region[T]` descriptor. Filesystem, device, and remote adapters are
  separate modules and are not implied by importing `std::region`.
- The core does not require asynchronous execution. If an adapter later adds a
  queue, its capacity, ownership, cancellation, timeout, and retry policy must
  be explicit and bounded.

## 6. Element layout contract

`Region[T]` accepts only a fully sized element type whose native size,
alignment, stride, and representation are deterministic.

The implementation must:

- reject unsized, incomplete, malformed, or unsupported generic elements;
- derive stride through the compiler layout contract;
- validate multiplication of logical index and stride before access;
- validate alignment for every resident buffer and provider transfer;
- preserve pack byte order, field offsets, padding, and width;
- preserve the distinction between a packed value and an ordinary struct;
- reject target layouts that cannot represent the declared contract;
- keep element layout stable across open, read, write, remap, publish, and
  recovery.

## 7. Generation and stale-state contract

Every mutable region state has a generation. Generations are monotonic and
must not wrap. A window transition, publication, cancellation, provider
recovery, or capability reuse advances or validates generation according to a
single documented state machine.

The implementation must define:

- which operations consume a generation token;
- which views and leases carry a generation;
- when a view becomes stale;
- whether a failed operation advances generation;
- how stale descriptors are rejected before touching resident bytes;
- how capability-slot reuse prevents old handles from addressing new data;
- what happens at generation exhaustion.

## 8. Concurrency and loan contract

The initial core may be single-threaded, but the data model must define how
multiple readers, one exclusive writer, pins, and provider transitions interact.
The contract must not rely on undocumented native locks.

Required behavior includes:

- immutable views may coexist only when the region state permits it;
- an `ins` operation excludes incompatible mutation and remap;
- a mutable loan cannot escape its call scope;
- a provider transition cannot invalidate a live loan silently;
- pinned windows cannot be evicted;
- a failed acquisition leaves existing owners valid;
- hosted and freestanding profiles expose the same semantic result classes.

If concurrency is not implemented in the first release, the unsupported state
must be a typed result and the roadmap must retain the later gate.

## 9. Failure and recovery contract

`RegionError` must cover the complete public failure domain without exposing
platform status codes. The taxonomy must distinguish at least:

- invalid descriptor or element layout;
- invalid or exhausted capability;
- stale generation or stale view;
- logical bounds and resident-window violations;
- offset, stride, alignment, and size overflow;
- buffer size and capacity errors;
- dirty, pinned, busy, or conflicting lifecycle state;
- provider unavailable, cancelled, timed out, or rejected;
- checksum, corruption, truncation, and incompatible generation;
- publication, rollback, and recovery failure.

Recovery rules must preserve the last accepted generation, keep failed dirty
state usable where possible, and never silently accept stale or partially
validated data.

## 10. ABI and lowering contract

The compiler and native backend must support Region descriptors through:

- direct calls;
- nested generic calls;
- pass-by-value and pass-by-reference paths;
- return values and caller-owned aggregate return slots;
- packed and ordinary fully sized element types;
- explicit close and lexical cleanup;
- success and every recoverable failure path.

The native lowering must not materialize logical capacity as an inline
aggregate. It must not introduce raw pointers into the public Actus value, and
it must preserve the exact ownership role at every call site.

## 11. Evidence contract

A Region capability is complete only with evidence for:

- semantic acceptance and rejection;
- formatter and source-limit compliance;
- native ABI and cleanup behavior;
- zero-floating-point behavior where the package requires it;
- bounded resident memory across increasing logical capacities;
- window remap, read, write, publish, cancel, and close;
- stale generation, capability reuse, and double-close rejection;
- layout, alignment, overflow, and malformed-element rejection;
- provider failures, interruption, corruption, retry, and recovery;
- deterministic hosted and freestanding profiles where supported;
- documented latency and allocation boundaries.

A successful semantic check alone does not prove native lowering, bounded
resident memory, provider recovery, or concurrency behavior.

## 12. Non-goals

This ADR does not define:

- a general-purpose garbage collector;
- implicit virtual memory or transparent page faults;
- a required filesystem format;
- a specific operating system, device, allocator, scheduler, or transport;
- raw pointer access for application code;
- unbounded collections or hidden allocation;
- application-specific data models;
- a claim that logical capacity equals usable physical capacity.

## 13. Compatibility and migration

The existing Phase 38 operations remain valid while new operations are added
behind explicit typed APIs. `Array[T, N]` behavior and ABI remain unchanged.
Existing Region descriptors must either remain readable under their declared
version or be rejected with a typed compatibility error. No silent conversion
from an older descriptor or generation is permitted.

## 14. Acceptance rule

The proposal is accepted only after every Phase 41 gate is implemented,
tested, documented, and backed by reproducible evidence. Open gates must remain
visible in the roadmap. A benchmark, compiler check, or successful example
cannot close a gate whose ownership, recovery, provider, or ABI evidence is
missing.
