# ADR-0073: Inline Aggregates and Runtime-Backed Logical Regions

## Status

Accepted.

## Context

Actus bounded arrays and aggregates describe physical inline storage. Their
const extent is part of the type, layout, and ABI contract. Making a large
inline value transparently page-backed would change contiguity, allocation,
cleanup, and call-boundary behavior without changing source syntax.

Large logical data sets require a different representation. Their logical
length can exceed resident memory, while only a bounded window is available to
the current operation. The compiler must therefore distinguish physical
inline storage from an explicit runtime-backed region instead of inferring a
storage policy from the magnitude of a const argument.

## Decision

### Inline aggregate contract

`Array[T, N]` and aggregates containing it remain inline values. Their storage
is contiguous, their layout is computed from the element layout and extent,
and their established direct or indirect native ABI is preserved. A large
extent does not silently convert an inline value into a page-backed object.

Inline aggregates may be rejected when the selected target cannot represent
their total physical size. The diagnostic must identify the checked size,
alignment, target profile, and the boundary that failed.

### Runtime-backed region contract

The first runtime-backed representation is a distinct generic region resource,
conceptually `Region[T]`. It is not an alternate lowering of `Array[T, N]`.
The public spelling and declarations will be introduced by a later runtime
implementation gate, but the semantic distinction and ABI boundary are fixed
by this ADR.

A region descriptor contains, directly or through a fixed-width capability
record:

- canonical element layout identity;
- checked element stride in bytes;
- logical element count;
- resident window start and count;
- page or window size;
- storage generation;
- lifecycle and dirty state;
- an opaque fixed-width storage capability.

The public and serialized capability representation is a fixed-width `u64`
handle. It identifies a slot in an Actus-managed capability table and is
target-agnostic; it is not an operating-system pointer, an address cast, or an
implicit allocation reference. A target may use a narrower internal slot index
only after a checked conversion, and that internal representation must never
leak into the public or serialized ABI. The descriptor has bounded size
independent of logical element count.

### Target address profiles

Every runtime-backed region selects an explicit address profile. The profile
declares two independent widths: the logical index width and the byte-offset
width. The first implementation accepts 32-bit or 64-bit widths for each
field. The hosted default is a 64-bit logical index and 64-bit byte offset; an
embedded or constrained target may select a 32-bit profile when both its
logical indexes and representable byte offsets fit that contract.

Region construction rejects unsupported widths, a logical element count whose
last index exceeds the selected index width, and a logical byte size whose
last byte exceeds the selected offset width. Index-to-byte multiplication,
window-end addition, element-end addition, and resident-storage sizing are
checked before conversion or access. A large logical capacity therefore does
not imply a large allocation: only the validated resident window is allocated
by the hosted test backend.

The address-width fields are fixed-width metadata in the `repr(C)` descriptor.
They are values, never pointers or target addresses, so the descriptor remains
position-independent when serialized by a later persistence implementation.

The runtime boundary exposes descriptor validation as a typed operation before
native access. Its ABI size and alignment are derived from the `repr(C)` type,
and the descriptor is passed as a bounded value rather than as an operating
system pointer. A public Actus `Region[T]` spelling and generated native
bridge remain a separate compiler gate; the hosted Rust boundary must not be
presented as if it were already an Actus source-level representation.

The capability table owns the association between the handle and the selected
storage backend. A handle is not sufficient to bypass lifecycle, bounds, or
generation validation, and a recycled slot must receive a new generation
before it can be observed by a later operation.

All region access is explicit. Opening, reading, mutating, publishing,
flushing, closing, and invalidating a region are separate operations with
typed success and failure results. The compiler must never insert filesystem
access, page loading, eviction, or allocation merely because a region value is
used.

### Ownership and view lifecycle

The owning region resource remains the authority for its descriptor and
capability. An `ins` operation may create a mutable resident view for the
duration of the call. The view is returned to the owner at the call boundary;
flush, publish, eviction, and reuse cannot occur while that view is live.

An `abs` operation may create a read-only call-scoped view. The view cannot
escape the operation that borrowed it. The first implementation does not
permit a view to be stored in a long-lived aggregate, returned as an
unrestricted resource, or transferred to another execution context.

`dat` transfers terminal ownership of a region resource only through an
explicit API that documents whether the capability remains valid after the
transfer. A failed operation preserves the owner and leaves the last
published generation unchanged unless the API explicitly documents a
consuming failure.

### Window and page rules

Every access validates logical index, window membership, byte-offset
multiplication, and end bound before touching storage. A request outside the
resident window returns a typed unavailable or bounds error; it does not
perform implicit I/O.

Dirty publication is explicit. A successful mutation marks the current
generation dirty. Flush validates the descriptor and generation before
publishing data. Eviction or reuse is rejected while an incompatible `ins` or
`abs` view is live. Failed flush, cancellation, or stale generation leaves the
previous published generation available and reports the failure.

Generation is a fixed-width monotonic `u64` counter. Generation `0` means
uninitialized and cannot authorize access. A successful publication or
replacement advances the counter before the new descriptor becomes visible.
Every view and operation carries the generation it observed; a mismatch is
rejected immediately with `StaleGeneration`. The counter never wraps. If the
next value cannot be represented, publication stops with a typed exhaustion
failure and the existing published generation remains valid.

### Concurrency profile

The first implementation uses a single-owner, serialized profile. A region
cannot be concurrently mutated by another task, core, or interrupt context.
The ownership checker and runtime state must reject such access explicitly;
ordinary `abs` sharing is not an implicit lock or pin protocol.

Multi-core read sharing, atomic descriptor updates, interrupt-safe windows,
and concurrent eviction require a separate concurrency decision and ABI
extension. They are not inferred from `abs` and are outside the first
implementation gate.

## Failure contract

The region API must expose typed failures for at least:

- invalid logical bounds;
- checked arithmetic or target address-width overflow;
- unavailable resident window;
- stale storage generation;
- invalid or exhausted capability;
- busy owner or live view;
- failed publication or backend capacity exhaustion.

No valid descriptor state may reach a compiler-generated trap. Invalid
descriptors must fail through the typed contract before an offset or backend
operation is attempted.

## Native and object contract

The region descriptor uses a compact, bounded ABI representation. Generated
code contains checked access and lifecycle calls, not one function or symbol
per logical element. Large zero-filled inline objects use the target's
declared zero-initialization section when supported; the compiler must not emit
a materialized zero-byte initializer. Non-zero initialization remains an
explicit bounded operation.

Mangled names, relocation metadata, descriptor records, and object metadata
must remain bounded as logical capacity grows. Inline aggregate ABI identity
and runtime-backed region ABI identity remain distinct at semantic, lowering,
and call boundaries.

## Alternatives rejected

- Reinterpreting a large `Array[T, N]` as paged storage: changes the existing
  contiguous layout and ABI without a source-level signal.
- Hidden heap allocation: violates deterministic ownership and makes resource
  cost invisible at the call boundary.
- Raw pointer descriptors: expose operating-system addresses and cannot provide
  position-independent or target-neutral serialized state.
- Implicit page faults or filesystem reads: add unbounded latency and hidden
  side effects to ordinary value access.
- Treating `abs` as a concurrency primitive: read-only visibility does not
  define pinning, generation consistency, or eviction synchronization.

## Consequences

Small and medium inline aggregates retain their current semantics and remain
cheap native values. Very large logical capacity becomes possible through an
explicit region API whose resident memory is bounded by its window policy.
Applications must choose the representation deliberately and handle typed
availability, generation, and publication failures.

The first implementation is intentionally serialized. This keeps ownership,
cleanup, and recovery deterministic while leaving a clear extension boundary
for target-specific synchronization and concurrent access.

## Verification required by Phase 37

- Semantic tests distinguish inline aggregate and runtime-backed region types.
- Layout tests verify stride, checked total size, descriptor size, and overflow.
- Address-profile tests verify 32-bit and 64-bit logical-index boundaries,
  checked byte-offset arithmetic, page crossing, last-element access, and
  fixed-width position-independent descriptor metadata.
- Native tests verify bounded symbols, sections, relocations, and zero
  initialization behavior.
- Runtime tests cover window bounds, dirty publication, stale generations,
  live-view eviction rejection, cancellation, and failed publication.
- Hosted and freestanding profiles report resident capacity separately from
  logical capacity.
- Capability reuse tests verify fixed-width handle validation and generation
  mismatch rejection, including generation exhaustion without wraparound.
