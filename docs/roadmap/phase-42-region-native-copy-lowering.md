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

- [ ] Add a minimal strict fixture using `Region[Array[u8, 16]]` with open,
      single-element read, single-element write, publish, and close.
- [ ] Add a range fixture using exact 128 KiB caller-owned buffers.
- [ ] Capture semantic, object-generation, link, and execution results
      separately.
- [ ] Distinguish compiler lowering failure, runtime descriptor validation,
      buffer bounds failure, ABI failure, and benchmark misuse.
- [ ] Record the exact diagnostic or native failure location for every rejected
      case.

## Gate 42.2 — Aggregate layout and stride lowering

- [ ] Lower `Region[Array[u8, 16]]` with size 16, the validated array
      alignment, and stride 16.
- [ ] Preserve nested array layout, field order, padding, and byte order for
      every supported fixed-size aggregate element.
- [ ] Use the same layout registry for semantic validation, Region opening,
      range bounds, native address calculation, and generated ABI signatures.
- [ ] Reject unsized, incomplete, malformed, or unsupported aggregate types
      before native code generation.
- [ ] Add layout tests for primitive, array, pack, and struct Region elements.
- [ ] Add negative tests for zero stride, invalid alignment, overflowed stride,
      and invalid logical byte extents.

## Gate 42.3 — Aggregate element read and write lowering

- [ ] Lower a single aggregate Region read into a checked address calculation
      followed by an exact-size copy into the caller destination.
- [ ] Lower a single aggregate Region write into a checked address calculation
      followed by an exact-size copy from the caller source.
- [ ] Preserve `abs`, `ins`, `erg`, and `dat` roles across aggregate operations.
- [ ] Reject wrong source or destination sizes before any byte changes.
- [ ] Verify that aggregate operations do not use scalar byte loops when a
      fixed-size native copy is available.
- [ ] Add direct, nested generic, returned, and ownership-forwarded aggregate
      Region fixtures.

## Gate 42.4 — Bounded bulk copy implementation

- [ ] Route `region_read_range` and `region_write_range` through one checked
      aggregate-aware bulk copy path.
- [ ] Accept an exact 128 KiB hosted payload and reject 128 KiB plus one byte
      with `BulkLimitExceeded` before copying.
- [ ] Use checked 64-bit arithmetic for element count, stride, byte count,
      relative offset, and range end.
- [ ] Keep the source and destination buffers caller-owned and reusable.
- [ ] Ensure overlapping ranges follow the documented copy semantics.
- [ ] Ensure failure is atomic: no partial destination, source, dirty-state, or
      generation change is visible after a rejected operation.
- [ ] Keep lower target profiles configurable without duplicating public APIs.

## Gate 42.5 — Native ABI and cleanup correctness

- [ ] Verify aggregate Region descriptors in direct function arguments and
      returns.
- [ ] Verify `Result[Region[T], RegionError]` with aggregate `T` in nested
      generic calls.
- [ ] Verify indirect return slots and aggregate temporary cleanup.
- [ ] Verify compiler-generated Region cleanup on early return, branch exit,
      nested scope exit, and failure paths.
- [ ] Verify that no raw pointer or provider handle enters a public aggregate,
      serialized value, or Region descriptor.
- [ ] Verify descriptor generation, stale handles, double close, and capability
      reuse remain unchanged.

## Gate 42.6 — Optimization and deterministic performance

- [ ] Eliminate redundant bounds checks only when the compiler can prove the
      same validated range and element layout.
- [ ] Replace constant-width division and multiplication sequences with safe
      strength-reduced operations where the target permits it.
- [ ] Use bounded native copy operations for aggregate elements and ranges.
- [ ] Keep optimization behavior deterministic across supported targets.
- [ ] Record compiler memory, object size, executable size, copy time, and peak
      resident memory for 64 KiB and 128 KiB transfers.
- [ ] Separate correctness measurements from performance measurements.

## Gate 42.7 — Cross-target and freestanding verification

- [ ] Run hosted Linux native fixtures.
- [ ] Run hosted Windows native fixtures.
- [ ] Run hosted macOS native fixtures.
- [ ] Emit supported freestanding objects and verify only documented provider
      bridge symbols are referenced.
- [ ] Verify aggregate layout and copy behavior on every supported pointer
      width and byte-order profile.
- [ ] Verify zero-floating-point output where the package contract requires it.
- [ ] Do not claim embedded latency, memory, or power behavior without a
      target-specific measurement.

## Gate 42.8 — Documentation and compatibility

- [ ] Update the Region API guide with aggregate element requirements and
      exact-size copy behavior.
- [ ] Update the runtime guide with the hosted 128 KiB profile and lower target
      profile rule.
- [ ] Update the Region ADR with the aggregate lowering and bulk-copy ABI.
- [ ] Document failure classes, atomicity, overlap semantics, and cleanup.
- [ ] Record the compiler revision, runtime revision, target, profile, commands,
      output, and known limits in a dated evidence document.
- [ ] Confirm that public Region operation names and ownership roles remain
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
