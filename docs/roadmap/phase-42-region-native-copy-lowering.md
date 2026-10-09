# Phase 42 — Region Aggregate Copy and Native Lowering

## Status

Proposed. This phase is dedicated to completing the native execution path for
aggregate Region elements and making bounded copy operations efficient,
checked, and target-neutral.

Phase 42 does not redefine `Array[T, N]`, `Region[T]`, ownership roles, or the
public Region operation names. It completes the compiler and runtime support
required for already-valid fixed-size aggregate element types, including
`Region[Array[u8, 16]]`.

No application-specific integration, device protocol, filesystem provider, or
model implementation belongs in this phase.

## Objective

Deliver one coherent native path for fixed-size aggregate Region elements:

```text
Region[Array[u8, 16]]
        -> validated element layout
        -> aggregate-aware native address calculation
        -> bounded caller-owned range buffer
        -> one checked bulk copy per range operation
```

The implementation must preserve explicit ownership, deterministic bounds
failures, zero hidden allocation on read/write paths, position-independent
data representation, and the configured zero-floating-point contract.

## Design contract

- `Region[T]` remains a runtime-backed capability with an opaque descriptor.
- `T` must be fully sized and have a validated native size and alignment.
- `Array[u8, 16]` has a 16-byte element stride and is copied as one aggregate
  value, not as sixteen unrelated public operations.
- Range operations use caller-owned buffers and a bounded profile limit.
- The hosted profile supports the current 128 KiB maximum bulk payload.
- A target profile may select a lower payload limit, including 64 KiB, without
  changing source-level Region APIs.
- No range operation may allocate implicitly, perform filesystem I/O, perform
  provider I/O, or create an unbounded temporary object.
- A failed validation must leave the destination, source, Region generation,
  dirty state, and published bytes unchanged.
- Native lowering must never materialize the logical Region extent as an inline
  array or one compiler object per logical element.

## Gate 42.1 — Reproduce and classify the native failure

- [x] Add a minimal strict fixture using `Region[Array[u8, 16]]` with open,
      single-element read, single-element write, publish, and close.
- [x] Add a range fixture using exact 128 KiB caller-owned buffers.
- [x] Capture semantic, object-generation, link, and execution results
      separately.
- [x] Distinguish compiler lowering failure, runtime descriptor validation,
      buffer bounds failure, ABI failure, and benchmark misuse.
- [x] Record the exact diagnostic or native failure location for every rejected
      case.

### Gate 42.1 evidence

- Semantic validation is covered by `aggregate_region_fixture_passes_semantic_analysis_separately`.
- Object generation is covered by `aggregate_region_object_emission_exposes_runtime_bridges`.
- Linking and execution are covered by `aggregate_region_lifecycle_is_native` and the
  bounded range fixtures.
- Aggregate buffer-size rejection is a runtime bounds result in
  `aggregate_region_rejects_wrong_buffer_without_dirtying_state` at
  `tests/region_aggregate_native.rs:225` and
  `aggregate_region_rejects_wrong_destination_size` at
  `tests/region_aggregate_native.rs:273`; both map to
  `RegionError::BufferTooSmall` at `src/runtime/region.rs:421` and
  `src/runtime/region.rs:456`.
- Bulk-limit rejection is a runtime `BulkLimitExceeded` path in
  `hosted_bulk_range_rejects_payload_above_the_limit` at
  `tests/region_aggregate_native.rs:315`, with the checked failure at
  `src/runtime/region.rs:160`.
- The lifecycle and range fixtures do not reproduce a linker failure; their rejected
  cases are typed runtime failures. A codegen audit did expose a compiler lowering
  defect: aggregate copies were emitted as scalar byte load/store sequences in
  `src/codegen/structs/memory.rs` and the Result lowering path. The defect was
  replaced with Cranelift's target-aware bounded memory-copy lowering and is covered
  by `native::lowers_fixed_aggregate_copies_through_target_aware_memory_copy`.
  No benchmark-specific workaround is part of this evidence.

**Gate status: Complete** — semantic validation, object generation, linking,
execution, runtime rejection classification, and diagnostic locations are
covered by the focused fixtures and runtime source references above.

## Gate 42.2 — Aggregate layout and stride lowering

- [x] Lower `Region[Array[u8, 16]]` with size 16, the validated array
      alignment, and stride 16.
- [x] Preserve nested array layout, field order, padding, and byte order for
      every supported fixed-size aggregate element.
- [x] Prove that the semantic fixed-size layout contract and native layout
      registry agree for Region opening, range bounds, native address
      calculation, and generated ABI signatures.
- [x] Reject unsized, incomplete, malformed, or unsupported aggregate types
      before native code generation.
- [x] Add layout tests for primitive, array, pack, and struct Region elements.
- [x] Add negative tests for zero stride, invalid alignment, overflowed stride,
      and invalid logical byte extents.

### Gate 42.2 evidence

- `region_layout_registry_stays_consistent_across_semantic_intrinsics_and_runtime_stride`
  in `tests/region_aggregate_native.rs` executes strict native code that checks
  `size_of` and `align_of` for primitive, fixed array, nested array, struct,
  array-of-struct, and packed layouts, then opens and closes an aggregate
  Region using the validated element stride.
- The fixture crosses semantic Region admission, native layout intrinsics,
  descriptor construction, runtime range metadata, and aggregate ABI lowering
  in one executable. It also exposed that nested array types used only by
  `size_of`/`align_of` were missing from native layout discovery; the compiler
  now registers those intrinsic type arguments before lowering.
- Existing layout coverage remains in `tests/codegen/layout.rs`, while generic
  Region ABI coverage remains in `tests/applications/runtime_modules.rs`.

**Gate status: Complete** — the semantic fixed-size contract, native layout
registry, Region descriptor stride, and aggregate native execution are covered
by executable consistency evidence.

## Gate 42.3 — Aggregate element read and write lowering

- [x] Lower a single aggregate Region read into a checked address calculation
      followed by an exact-size copy into the caller destination.
- [x] Lower a single aggregate Region write into a checked address calculation
      followed by an exact-size copy from the caller source.
- [x] Preserve `abs`, `ins`, `erg`, and `dat` roles across aggregate operations.
- [x] Reject wrong source or destination sizes before any byte changes.
- [x] Verify that aggregate operations do not use scalar byte loops when a
      fixed-size native copy is available.
- [x] Add direct, nested generic, returned, and ownership-forwarded aggregate
      Region fixtures.

### Gate 42.3 evidence

- `native::lowers_fixed_aggregate_copies_through_target_aware_memory_copy` in
  `tests/codegen.rs` verifies that a fixed aggregate assignment lowers through
  the target-aware native copy path rather than a scalar byte loop.
- `generic_region_covers_array_struct_and_aggregate_return_abi` in
  `tests/applications/runtime_modules.rs` executes direct aggregate Region
  operations through generic open and forwarding verbs, aggregate returns,
  ownership transfer, and array, struct, and byte-array element types.
- The focused tests passed after the copy-lowering change; the full test suite
  remains the acceptance check for the unchanged scalar and aggregate paths.

**Gate status: Complete** — aggregate element reads and writes, role
preservation, size rejection, target-aware copy lowering, and generic ABI
fixtures are covered by executable evidence.

## Gate 42.4 — Bounded bulk copy implementation

- [x] Route `region_read_range` and `region_write_range` through one checked
      aggregate-aware bulk copy path.
- [x] Accept an exact 128 KiB hosted payload and reject 128 KiB plus one byte
      with `BulkLimitExceeded` before copying.
- [x] Use checked 64-bit arithmetic for element count, stride, byte count,
      relative offset, and range end.
- [x] Keep the source and destination buffers caller-owned and reusable.
- [x] Ensure overlapping ranges follow the documented copy semantics.
- [x] Ensure failure is atomic: no partial destination, source, dirty-state, or
      generation change is visible after a rejected operation.
- [x] Keep lower target profiles configurable without duplicating public APIs.

### Gate 42.4 evidence

- `RegionView::read_range` and `MutableRegionView::write_range` both validate
  the same descriptor byte range and exact caller-buffer length before calling
  the shared `copy_bulk_bytes` primitive in `src/runtime/region.rs`.
- `runtime::region::tests::bounded_range_access_is_atomic_and_supports_empty_and_full_windows`
  and the native `std_region_native` range fixtures exercise the shared path;
  the exact 128 KiB boundary remains covered by the existing hosted test.
- `RegionDescriptor::byte_range_for` uses checked addition and multiplication
  for index, stride, offset, byte count, and range end, then applies the target
  address profile and bulk limit before any copy. The boundary matrix and
  overflow tests cover the rejected arithmetic paths.
- The range APIs accept caller-provided slices and do not allocate or retain
  them; the native fixtures reuse caller-owned buffers across read and write
  operations.
- `bulk_copy_preserves_memmove_overlap_semantics` verifies the overlap-safe
  `ptr::copy` contract used by the shared primitive.
- `failed_range_validation_preserves_buffers_and_region_state` verifies that
  rejected writes leave staged bytes and dirty state unchanged, while rejected
  reads leave the caller destination unchanged.
- `target_profile_can_lower_the_bulk_limit_without_changing_region_operations`
  verifies a 64-byte target limit alongside the unchanged public read/write
  operations; the hosted default remains 128 KiB.

**Gate status: Complete** — bounded range arithmetic, shared copying, caller
buffer ownership, overlap behavior, atomic failures, and lower target limits
are covered by runtime and native evidence.

## Gate 42.5 — Native ABI and cleanup correctness

- [x] Verify aggregate Region descriptors in direct function arguments and
      returns.
- [x] Verify `Result[Region[T], RegionError]` with aggregate `T` in nested
      generic calls.
- [x] Verify indirect return slots and aggregate temporary cleanup.
- [x] Verify compiler-generated Region cleanup on early return, branch exit,
      nested scope exit, and failure paths.
- [x] Verify that no raw pointer or provider handle enters a public aggregate,
      serialized value, or Region descriptor.
- [x] Verify descriptor generation, stale handles, double close, and capability
      reuse remain unchanged.

### Gate 42.5 evidence

- `returns_transitive_aggregate_values_through_native_slots` in
  `tests/arrays_cli.rs` executes struct, pack, fixed array, and const-generic
  aggregate values through direct and transitive indirect return slots. The
  same executable consumes the returned temporaries, covering their cleanup
  path.
- `hosted_std_region_cleanup_handles_early_return_and_nested_transfer` in
  `tests/std_region_native.rs` now exercises early return, branch exit, nested
  ownership transfer, and a typed read failure that unwinds an owned Region.
- `RegionDescriptor` is a fixed-width `repr(C)` value containing only integer
  handles, lengths, offsets, generations, flags, and profile widths; the
  descriptor layout test verifies its 64-byte ABI. Provider state remains
  behind runtime tables and is never part of the public descriptor.
- `runtime::region_bridge_tests` and `runtime::capabilities::tests` cover stale
  generations, failed and repeated close, generation reuse, wraparound, and
  unknown-handle rejection.

**Gate status: Complete** — aggregate return slots, cleanup paths, descriptor
representation, generation validation, and capability reuse are covered by
native and runtime evidence.

## Gate 42.6 — Optimization and deterministic performance

- [ ] Eliminate redundant bounds checks only when the compiler can prove the
      same validated range and element layout.
- [ ] Replace constant-width division and multiplication sequences with safe
      strength-reduced operations where the target permits it.
- [x] Use bounded native copy operations for aggregate elements and ranges.
- [ ] Keep optimization behavior deterministic across supported targets.
- [x] Record compiler memory, object size, executable size, copy time, and peak
      resident memory for 64 KiB and 128 KiB transfers.
- [x] Separate correctness measurements from performance measurements.

### Gate 42.6 evidence

- `docs/benchmarks/phase-42-region-performance-2026-10-09.md` records the
  optimized hosted benchmark for 64 KiB and 128 KiB read/write operations.
- Correctness remains covered by the runtime and native Region suites; the
  performance record is kept separate from those acceptance tests.
- Aggregate element and range operations use bounded target-aware copy paths;
  no unbounded copy primitive is introduced by this phase.
- The dated benchmark record includes compiler peak RSS, object and executable
  sizes, build time, and runtime copy time for the 64 KiB and 128 KiB cases.

### Gate 42.6 optimization audit

The two remaining optimization checks stay open deliberately. `RegionDescriptor`
receives the logical index, element count, window bounds, element stride, and
bulk limit at runtime. The compiler therefore cannot currently prove that the
validated range remains unchanged across the copy call, so removing a second
check would weaken the safety contract. The stride is also descriptor data
rather than a compile-time constant for every generic `Region[T]` instance;
replacing its checked arithmetic with target-specific strength reduction needs
an explicit compiler proof and cross-target codegen evidence. No wrapper,
unchecked arithmetic, or target-specific exception is used to close either
check.

## Gate 42.7 — Cross-target and freestanding verification

- [x] Run hosted Linux native fixtures.
- [ ] Run hosted Windows native fixtures.
- [ ] Run hosted macOS native fixtures.
- [x] Emit supported freestanding objects and verify only documented provider
      bridge symbols are referenced.
- [x] Verify aggregate layout and copy behavior on every supported pointer
      width and byte-order profile.
- [x] Verify zero-floating-point output where the package contract requires it.
- [x] Do not claim embedded latency, memory, or power behavior without a
      target-specific measurement.

### Gate 42.7 execution plan

The repository CI matrix runs the dedicated `std_region_native` suite on
`ubuntu-latest`, `macos-latest`, and `windows-latest`. The suite covers hosted
native Region execution, exact 128 KiB transfers, aggregate cleanup, bounded
logical capacity, freestanding object emission, provider-bridge symbol
boundaries, and the zero-floating-point verification policy. The workflow now
publishes a separate Region gate summary for each operating system. The Gate
42.7 checkboxes remain open only for the macOS and Windows hosted runs until
that matrix has produced passing evidence for the current revision.

The Linux run passed the dedicated `std_region_native` suite, including hosted
native fixtures, freestanding object emission, provider-bridge symbol checks,
and zero-floating-point verification. Runtime coverage now also preserves an
aggregate byte pattern under both 32-bit and 64-bit Region address profiles,
while target coverage parses little-endian 32-bit and big-endian 32-bit and
64-bit target contracts.

## Gate 42.8 — Documentation and compatibility

- [x] Update the Region API guide with aggregate element requirements and
      exact-size copy behavior.
- [x] Update the runtime guide with the hosted 128 KiB profile and lower target
      profile rule.
- [x] Update the Region ADR with the aggregate lowering and bulk-copy ABI.
- [x] Document failure classes, atomicity, overlap semantics, and cleanup.
- [ ] Record the compiler revision, runtime revision, target, profile, commands,
      output, and known limits in a dated evidence document.
- [x] Confirm that public Region operation names and ownership roles remain
      source-compatible.

## Gate 42.9 — Final acceptance

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo check --all-targets --all-features`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run source-limit and diff validation.
- [ ] Run Actus formatter, strict checks, aggregate native fixtures, bulk
      transfer fixtures, and zero-float verification.
- [ ] Verify that all gates have implementation and reproducible evidence.
- [ ] Mark Phase 42 complete only after the aggregate native path and bounded
      bulk copy contract pass every required target profile.

## Definition of done

Phase 42 is complete when `Region[Array[u8, 16]]` executes natively through
its complete lifecycle, aggregate reads and writes use validated fixed-size
copy paths, hosted 128 KiB range transfers pass without hidden allocation, the
lower target profile remains explicit, and all compiler, runtime, ABI,
ownership, cleanup, documentation, and cross-target evidence is reproducible.

## Explicit non-goals

- No implicit paging, filesystem access, device access, or provider scheduling.
- No automatic conversion between `Array[T, N]` and `Region[T]`.
- No unbounded bulk payload or hidden heap growth.
- No application-specific data model or protocol.
- No unsupported hardware performance claim based on hosted execution.
