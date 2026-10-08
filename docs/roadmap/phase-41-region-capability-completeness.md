# Phase 41 — Complete `std::region` Capability and Windowing

## Status

Proposed. This phase is dedicated exclusively to making `std::region` a
complete, target-neutral, bounded logical-storage library. No downstream
application, device, model, or product integration belongs in this phase.

Phase 38 remains the completed foundation. Phase 41 extends and hardens that
foundation; it does not redefine `Array[T, N]`. The internal Region descriptor
ABI is versioned when its validated layout contract expands; public Region
operation names and ownership roles remain source-compatible.

The governing agreement is [ADR-0079](../decisions/ADR-0079-region-capability-completeness.md).

## Objective

Deliver a production-grade Region abstraction that can represent a large
logical index space through a bounded resident window with explicit movement,
checked access, deterministic ownership, typed provider failures, generation
safety, cleanup, recovery, and reproducible native evidence.

## Priority rule

Until this phase is complete, no unrelated standard-library feature or
application integration advances. Every gate below is an evidence gate. A
checkbox may be marked only after implementation, tests, documentation, and
reproducible evidence support the exact claim.

## Gate 41.1 — Contract inventory and compatibility baseline

- [x] Accept ADR-0079 and cross-reference it from the standard-library index.
- [x] Inventory every existing public Region declaration, bridge, error, test,
      guide section, and runtime manifest entry.
- [x] Record the Phase 38 ABI, lifecycle, layout, and compatibility guarantees.
- [x] Define the versioning policy for Region descriptors and provider results.
- [x] Define which existing operations remain source- and binary-compatible.
- [x] Add a negative contract list for unsupported implicit behavior.

#### Gate 41.1 evidence

The inventory is recorded in the `std::region` section of the standard-library
API index and in ADR-0079 section 2.1. The current implementation inventory is:

- Public facade: `library/std/src/region/region.act`.
- Public error taxonomy: `library/std/src/region/error.act`.
- Public typed operations and private unsafe bridges:
  `library/std/src/region/api.act`.
- Compiler-owned runtime model: `src/runtime/region.rs` and
  `src/runtime/region_bridge.rs`.
- Runtime symbol contract: `src/runtime/contract.rs`.
- Capability table: `src/runtime/capabilities.rs`.
- Semantic coverage: `tests/std_region_semantic.rs` and
  `tests/semantic/generics.rs`.
- Native and cleanup coverage: `tests/std_region_native.rs`,
  `tests/codegen/native.rs`, and `tests/applications/runtime_modules.rs`.
- Logical-capacity evidence: `tests/std_region_scale.rs` and the Phase 38
  scale evidence document.
- Language and ownership guidance: the Region section of
  `docs/guide/README.md` and the standard-library API index.

The compatibility policy is versioned by the public Region descriptor and
provider-result contract. Existing declarations remain source-compatible;
descriptor or provider-result changes require an explicit version and typed
compatibility failure. No silent descriptor conversion is allowed. Public
operations remain explicit and versioned; Gate 41.10 adds `region_pin`,
`region_unpin`, and `region_pinned` with their own contract and evidence.

The negative contract list is explicit: no implicit Array conversion, hidden
allocation, implicit page fault, implicit filesystem or device I/O, raw pointer
in public values, unbounded collection, silent generation repair, automatic
remap, or provider-specific type in the public facade.

## Gate 41.2 — Descriptor representation and capability identity

- [x] Define the target-agnostic public descriptor fields and their widths.
- [x] Define capability-slot allocation, reuse, and exhaustion behavior.
- [x] Add a monotonic generation counter that cannot silently wrap.
- [x] Reject invalid, stale, closed, and reused capabilities before data access.
- [x] Prove that public descriptors contain no raw OS or provider pointers.
- [x] Specify pass-by-value, pass-by-reference, and return ABI behavior.
- [x] Add direct and nested generic descriptor identity tests.

#### Gate 41.2 evidence

The identity profile is defined in ADR-0079 section 4.1. The implementation
uses one shared handle encoder and decoder in `src/runtime/capabilities.rs`
and `src/runtime/region_bridge.rs`; the hosted bridge no longer maintains a
second handle encoding formula.

The descriptor was `repr(C)`, 56 bytes, aligned to 8 bytes, and contained only
fixed-width integer fields. The ABI version was `1`. Gate 41.3 supersedes that
internal descriptor profile with an explicitly versioned layout. Capability slot reuse
increments the upper 32-bit generation and rejects the old handle. Slot
capacity and generation wraparound return typed failures.

Evidence commands:

```text
cargo fmt --all -- --check
cargo test --lib runtime::capabilities -- --test-threads=1
cargo test --lib runtime::region -- --test-threads=1
cargo test --test std_region_semantic --test std_region_native -- --test-threads=1
```

Evidence result: formatter passed; 6 capability tests passed; 11 runtime
Region and cleanup tests passed; 2 semantic facade tests and 7 native ABI,
freestanding, lifecycle, and zero-floating-point tests passed. No public
descriptor pointer or provider-specific type was introduced.

## Gate 41.3 — Element layout, stride, alignment, and overflow

- [x] Accept supported sized primitives, arrays, packs, and structs.
- [x] Reject unsized, incomplete, malformed, and unsupported element types.
- [x] Validate size, alignment, stride, logical length, window count, and byte
      offset before native access.
- [x] Detect index-times-stride and range-end overflow deterministically.
- [x] Preserve pack layout, byte order, padding, and field offsets.
- [x] Add accepted and rejected layout fixtures for hosted and freestanding
      profiles.

#### Gate 41.3 evidence

The compiler now provides `align_of[T]()` alongside `size_of[T]()` for fixed
layout types. `region_open` derives both values and passes them through the
private bridge. The runtime descriptor is version 3, 64 bytes, and stores the
validated element alignment at offset 16; bridge generation and dirty writes
use the updated offsets at 48 and 56.

Runtime validation rejects zero, non-power-of-two, and over-stride alignment,
zero or overflowing extents, invalid windows, unsupported address widths, and
index-times-stride or range-end overflow. Existing primitive and packed
layouts retain their field offsets and byte order. Semantic fixtures cover
accepted fixed-size types and rejected unsized or missing intrinsic arguments;
runtime fixtures cover valid packed alignment and invalid descriptor profiles.

Evidence commands and results are recorded after the gate implementation is
validated:

```text
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo test --lib runtime::region -- --test-threads=1
cargo test --test semantic_intrinsics --test std_region_semantic --test std_region_native -- --test-threads=1
```

## Gate 41.4 — Complete window lifecycle

- [x] Define and implement explicit window inspection.
- [x] Define and implement explicit window remap or replacement.
- [x] Reject remap while a conflicting `ins` loan, dirty transaction, or pin
      is active.
- [x] Define window replacement and admission results without implicit I/O;
      provider-mediated load and eviction follow the Gate 41.8 provider
      boundary.
- [x] Preserve the previous valid generation when a transition fails.
- [x] Test the initial window, forward movement, boundary rejection, dirty
      rejection, generation advancement, and invalid movement.

#### Gate 41.4 evidence

The public facade now exposes read-only metadata inspection and
`region_remap`. The hosted runtime validates logical range, checked
`window_count * element_stride`, exact backing capacity, current generation,
and dirty state before replacing resident storage. Successful remap updates
window metadata and generation atomically from the caller's perspective;
failed remap leaves the old window and generation unchanged.

Evidence includes runtime tests for clean remap, dirty rejection, invalid
window preservation, and generation advancement, plus hosted strict native
execution covering inspection, remap, close, and zero-floating-point IR.

## Gate 41.5 — Element and bounded range access

- [x] Keep single-element read and write checked and allocation-free.
- [x] Add bounded range read and write with caller-owned buffers.
- [x] Define overlap semantics for range operations.
- [x] Define partial-progress and failure reporting.
- [x] Reject ranges crossing the resident window unless an explicit staged
      operation is used.
- [x] Add empty, one-element, full-window, boundary, and overflow tests.

#### Gate 41.5 evidence

The public facade now exposes `region_read_range` and `region_write_range`.
Both operations require caller-owned buffers with exactly
`element_count * size_of[T]()` bytes. The runtime validates the complete
logical range, resident-window containment, checked byte extent, and buffer
size before copying. Successful operations return the complete `u64` element
count; failures return typed errors with zero progress. Native copies use
memmove semantics for any overlapping storage boundary, and empty ranges are
bounded no-ops. No allocation, filesystem operation, provider call, or
implicit remap is performed.

Evidence includes runtime tests for empty, one-element, full-window,
cross-window, wrong-size, and overflow cases; semantic facade export tests;
and a hosted strict native fixture covering range write, publication, range
read, empty read, boundary rejection, and zero-floating-point IR.

## Gate 41.6 — Ownership, views, loans, and deterministic cleanup

- [x] Specify `erg`, `abs`, `dat`, and `ins` for every Region operation.
- [x] Make views generation-aware and non-escaping.
- [x] Reject use after close, use after move, stale views, and aliasing loans.
- [x] Generate exactly one cleanup action for every owned descriptor.
- [x] Prove explicit close, failed close, lexical drop, early return, and nested
      call cleanup behavior.
- [x] Verify that failed recoverable operations preserve the owner.

#### Gate 41.6 evidence

The public Region contract now records the role at every boundary: `dat` for
opening and remapping backing storage, `abs` for read-only Region and source
views, `ins` for exclusive mutation, publication, cancellation, and close, and
`erg` for scalar indexes, counts, and layout values. Hosted views carry the
descriptor generation through every borrow; Rust lifetimes keep the views
non-escaping and prevent remap, close, or publication while a conflicting view
is live. The bridge validates the descriptor pointer, capability identity, and
generation before access.

Native Region values are descriptor pointers. The compiler cleanup ABI now
passes that pointer directly to `actus_region_drop`; it does not reinterpret a
pointer as an encoded capability handle. The bridge locates the descriptor in
the bounded capability table, releases its resident storage and descriptor
exactly once, and rejects repeated or unknown cleanup. Explicit close removes
the live resident capability while leaving one lexical descriptor cleanup;
failed close leaves the owner and capability intact.

Evidence includes runtime tests for failed-close owner preservation and
repeated release, compiler lowering coverage for direct Region cleanup, and a
strict hosted native fixture covering lexical drop, early return, nested
ownership transfer, use after close, stale generations, and zero-floating-point
IR. The fixture uses explicit `dat` at each ownership-transfer call site.

## Gate 41.7 — Publication, cancellation, and bounded transactions

- [x] Define dirty-state transitions and publication preconditions.
- [x] Implement generation advancement only after accepted publication.
- [x] Implement cancellation to the last accepted resident generation.
- [x] Define bounded transaction or staging semantics for multi-element updates.
- [x] Reject partial or invalid publication without losing the live generation.
- [x] Add repeated publish, cancel, failure, and retry evidence.

#### Gate 41.7 evidence

The hosted Region backend now treats resident storage as a bounded transaction
staging area and keeps a same-sized accepted snapshot. A successful publication
validates the live capability, generation, descriptor, and both storage extents
before copying the complete resident window, clearing `dirty`, and advancing
the generation exactly once. Cancellation restores the complete accepted
snapshot, clears `dirty`, and keeps the current generation unchanged. Failed
publication caused by a stale generation or generation exhaustion leaves the
generation, dirty state, staged bytes, and accepted snapshot available for
retry or cancellation. No filesystem, provider, or implicit paging operation is
performed by this memory-only transition.

Runtime evidence covers multi-element atomic publication, full-window
cancellation, repeated publish/cancel, stale-generation retry, generation
exhaustion preservation, invalid publication storage, and the existing native
publish/cancel fixtures. Focused validation passed with 19 Region unit tests
and a warning-free all-target clippy run.

## Gate 41.8 — Provider-neutral storage boundary

- [x] Define the target-neutral provider contract for load, store, flush, and
      recovery.
- [x] Add a deterministic memory-only provider for unit and native tests.
- [x] Define explicit provider results for unavailable, cancelled, timed out,
      corrupt, truncated, rejected, and retryable operations.
- [x] Keep provider-specific types and pointers outside the public Region type.
- [x] Prove that core Region operations do not perform hidden filesystem or
      device I/O.
- [x] Define the ownership of provider buffers and queued work.

#### Gate 41.8 evidence

The runtime now exposes a target-neutral `RegionProvider` contract with fixed
request metadata and explicit load, store, flush, cancel, and recover methods.
`MemoryRegionProvider` supplies one bounded accepted snapshot and one bounded
staging buffer for deterministic tests. Provider failures distinguish invalid
requests, capacity errors, unavailable, cancelled, timed-out, corrupt,
truncated, rejected, and retryable states; the public Region descriptor remains
free of provider-specific types and pointers. Provider buffers are caller-owned
at load/store boundaries, while provider snapshots remain bounded provider
state. The core has no implicit provider, filesystem, device, or asynchronous
queue operation.

Focused evidence covers staging versus accepted bytes, cancellation and
recovery, explicit timeout preservation, and wrong-capacity rejection. The
existing Region tests continue to exercise memory-only read, write, remap,
publish, cancel, and close behavior.

## Gate 41.9 — Recovery, integrity, and generation continuity

- [x] Define checksum or integrity validation at the provider boundary.
- [x] Preserve the previous complete generation through interrupted publish.
- [x] Reject stale, truncated, mismatched, and partially validated state.
- [x] Define retry, rollback, read-only degradation, and terminal failure.
- [x] Verify dirty resident state after failed publication or cancellation.
- [x] Add corruption, interruption, retry, and recovery fixtures.

#### Gate 41.9 evidence

The provider boundary now records CRC32 checksums for accepted and staged
buffers and validates them before load, recovery, and flush. Accepted and
staged generations are tracked independently; a timed-out or rejected flush
leaves the previous complete generation readable and the staged operation
retryable. Stale and exhausted generations, checksum mismatches, truncated
buffers, and invalid capacity are rejected without mutating accepted state.
Read-only degradation and terminal failure are explicit provider states.

Eight focused provider tests cover checksum and truncation rejection,
interrupted publication, retry, rollback, stale/exhausted generations,
read-only behavior, terminal failure, and accepted-state preservation. CRC32 is
documented as integrity detection only; authentication and encryption remain
outside this gate.

## Gate 41.10 — Concurrency and pinning contract

- [x] Define the initial single-threaded guarantee explicitly.
- [x] Define immutable reader coexistence and exclusive writer rules.
- [x] Define pin and unpin behavior and eviction restrictions.
- [x] Reject remap and explicit close while a window is pinned; active loans
      are rejected by the call-scoped `ins`/`abs` ownership contract.
- [x] Define unsupported concurrent operations as typed `WindowBusy` results.
- [x] Add deterministic state-machine tests for readers, writers, pins, and
      provider transitions.

#### Gate 41.10 evidence

The initial Region runtime is deliberately single-threaded at the semantic
boundary. An `abs` operation is a call-scoped immutable view, while `ins` is a
call-scoped exclusive mutable loan. Rust lifetime checks prevent a remap,
publication, cancellation, or close from executing while either view remains
live. Unsupported conflicting transitions are represented by typed
`WindowBusy` results.

The fixed 64-byte descriptor uses the previously unused byte at offset 59 for
the `pinned` state and retains four reserved bytes. Pin and unpin preserve the
generation and resident bytes. A pinned window may be published, but remap and
explicit close are rejected until unpinned. Provider eviction must treat the
same state as non-evictable. Repeated unpin is idempotent, and invalid pin
states fail descriptor validation.

Runtime tests cover loan lifetime exclusion, pin/remap/publication transitions,
idempotent unpinning, descriptor layout, invalid pin state, and provider state
transitions. Public facade tests cover the `region_pin`, `region_unpin`, and
`region_pinned` operations. Multi-threaded execution and reader coexistence
remain outside this initial contract and require a separately versioned design.

Validation evidence:

```text
target/debug/actus lock --check
target/debug/actus fmt --check library/std/src/region/api.act
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

All commands passed. The full suite includes 11 native Region fixtures, 2
public Region semantic tests, 9 provider tests, and the complete repository
test matrix. Native Region fixtures also report zero floating-point IR.

## Gate 41.11 — Bulk and performance behavior

- [ ] Define bounded bulk operation sizes and queue limits.
- [ ] Measure resident bytes independently from logical capacity.
- [ ] Measure remap, read, write, publish, cancel, and close costs separately.
- [ ] Record compiler memory, object size, executable size, and symbol effects.
- [ ] Prove that increasing logical length does not materialize one element per
      logical position in compiler IR or native artifacts.
- [ ] Record allocation and I/O behavior for every hot and provider path.

## Gate 41.12 — Native lowering and target profiles

- [ ] Cover primitive, array, pack, and struct Region elements in native ABI
      tests.
- [ ] Cover nested generic calls, aggregate returns, and indirect return slots.
- [ ] Cover hosted and supported freestanding profiles.
- [ ] Verify no raw pointer leaks into public values or serialized state.
- [ ] Verify zero-floating-point output where the package contract requires it.
- [ ] Add target-specific behavior only behind documented provider profiles.

## Gate 41.13 — Diagnostics, formatter, LSP, and guide completeness

- [ ] Document every public type, error, constant, and operation.
- [ ] Add actionable diagnostics for layout, bounds, generation, capability,
      loan, provider, and recovery failures.
- [ ] Preserve Region declarations through formatter round trips.
- [ ] Add semantic model, hover, completion, and source-navigation coverage.
- [ ] Add examples for inline arrays, one resident window, remap, bulk access,
      publication, cancellation, and recovery.
- [ ] Keep all source and documentation files within repository limits.

## Gate 41.14 — Adversarial and regression verification

- [ ] Add accepted and rejected semantic tests for every public constraint.
- [ ] Add native execution tests for every lifecycle state and failure branch.
- [ ] Add deterministic fuzz or property-style coverage for indices, lengths,
      windows, generations, and malformed provider results.
- [ ] Add double-close, stale-handle, stale-view, generation-wrap, and
      capability-reuse regressions.
- [ ] Add layout and ABI regressions for every supported element category.
- [ ] Run the full compiler and standard-library quality matrix.

## Gate 41.15 — Scale and evidence package

- [ ] Run logical capacities from MiB through TiB with fixed resident windows.
- [ ] Run multi-window remap workloads without hidden allocation or implicit I/O.
- [ ] Record exact compiler revision, target, profile, commands, peak RSS,
      resident bytes, artifact sizes, timings, and failure boundaries.
- [ ] Publish reproducible benchmark instructions and captured output.
- [ ] Document supported limits and every intentional deferral.
- [ ] Review the evidence independently against ADR-0079.

## Gate 41.16 — Final acceptance and release readiness

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo check --all-targets --all-features`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run source-limit and diff validation.
- [ ] Run Actus formatter, strict checks, native builds, and Region fixtures.
- [ ] Confirm no unrelated standard-library or application work was included.
- [ ] Update the guide, API index, ADR, and evidence documents.
- [ ] Mark Phase 41 complete only when every gate has implementation and
      reproducible evidence.

## Explicit non-goals

- No implicit page faults or hidden synchronous filesystem access.
- No raw pointer or operating-system handle in public Region values.
- No unbounded storage, garbage collection, or automatic dynamic collection.
- No change to the meaning or ABI of `Array[T, N]`.
- No application-specific storage model.
- No unsupported target claim based only on hosted execution.

## Definition of done

Phase 41 is complete when `std::region` provides a documented and tested
logical-storage contract with explicit window movement, bounded access,
provider-neutral lifecycle, generation-safe capabilities, deterministic
cleanup, typed recovery, native ABI evidence, and reproducible scale results.
