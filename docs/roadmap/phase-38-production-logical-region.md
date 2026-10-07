# Phase 38 — Production Logical Region

## Objective

Complete the generic `Region[T]` language and standard-library contract so a
program can represent a large logical storage space with a bounded resident
window, checked access, deterministic ownership, and restart-safe publication.

This phase is compiler and standard-library work. It contains no application
model, device model, product terminology, or downstream project integration.

## Non-goals

- Do not change the meaning of `Array[T, N]`.
- Do not infer a `Region[T]` from a large array automatically.
- Do not add hidden allocation, implicit filesystem access, raw pointers, or
  implicit page faults.
- Do not add target-specific behavior to the language or public Region API.
- Do not claim that logical capacity is resident memory capacity.

## Representation contract

- `Array[T, N]` remains contiguous inline storage with its existing ABI.
- `Region[T]` is a distinct owned resource with a checked descriptor and a
  bounded resident window.
- The element type must be fully sized and its stride must be derived by
  `size_of[T]()`.
- Public capability and generation fields use fixed-width integer values.
- Logical length, window start, window count, stride, alignment, and byte
  offsets are validated before access.

## Gates

### Gate 38.1 — Public API and documentation contract

- [x] Document `Region[T]`, `RegionError`, and every public Region operation in
      the language guide.
- [x] Document the difference between inline arrays and runtime-backed regions.
- [x] Document `dat`, `erg`, `abs`, and `ins` behavior for every operation.
- [x] Document bounds, stale-generation, capability, buffer-size, and backend
      failures without exposing native status values.
- [x] Keep the guide free of downstream application and device terminology.

### Gate 38.2 — Semantic validation

- [x] Accept sized primitive, array, pack, and struct element types.
- [x] Reject unsized, incomplete, malformed, and multi-parameter Region types.
- [x] Reject invalid logical lengths, windows, strides, alignments, and target
      address-width combinations before native lowering.
- [x] Preserve the Region type identity through generic calls and facade
      imports.
- [x] Add accepted and rejected semantic tests for every public constraint.

### Gate 38.3 — Native ABI and cleanup

- [x] Keep the Region descriptor layout fixed-width, pointer-free, and stable.
- [x] Preserve the descriptor contract through direct and nested generic calls.
- [x] Generate exactly one cleanup action for every owned Region.
- [x] Release a capability exactly once after successful close or lexical drop.
- [x] Preserve the owner after failed close, publish, or cancellation.
- [x] Add native execution tests for success, failure, nested calls, and early
      return cleanup.

### Gate 38.4 — Window lifecycle

- [x] Validate open, read, write, publish, cancel, and close as one lifecycle.
- [x] Reject stale generations immediately and prevent generation wraparound.
- [x] Prevent incompatible publication, eviction, or reuse while a view is
      live.
- [x] Keep dirty resident bytes usable after failed publication.
- [x] Prove that cancellation restores the last published resident bytes.
- [x] Keep all access explicit and free of implicit filesystem operations.

### Gate 38.5 — Large logical capacity evidence

- [x] Check and build Region sources with logical lengths of 1 MiB, 1 GiB, and
      1 TiB without materializing one object per logical element.
- [ ] Record compiler RSS, object size, symbol size, and build time for the
      supported large logical-capacity cases.
- [x] Execute checked reads and writes within a bounded resident window.
- [x] Prove that out-of-window and overflowed access fails deterministically.
- [x] Prove that resident memory remains bounded as logical length increases.

### Gate 38.6 — Quality and release acceptance

- [x] Run formatter, source-limit, compiler, semantic, native, and documentation
      checks.
- [x] Run the full test suite with no filtered failures.
- [x] Verify no floating-point instructions are introduced by Region lowering.
- [x] Verify no reverse pipeline dependency or backend type leaks into frontend
      or standard-library contracts.
- [x] Update the language guide and roadmap evidence with only implemented
      generic behavior.
- [x] Record exact compiler revision and reproducible commands.

### Gate 38.7 — Packed element native lowering

- [x] Accept a fully sized `pack` as the element type of `Region[T]`.
- [x] Resolve the packed element's canonical native representation during
      generic specialization instead of treating it as an unknown native type.
- [x] Derive and preserve the packed element size, alignment, stride, byte
      order, and declared field offsets through Region open, read, and write.
- [x] Lower `region_open[T]`, `region_read[T]`, `region_write[T]`, publish,
      cancel, and close for packed `T` without raw pointers or hidden
      allocation.
- [x] Preserve the packed element contract through direct calls, nested
      generic calls, and canonical facade imports.
- [x] Return a diagnostic that identifies the element type and required layout
      when a packed Region element is malformed or unsupported.
- [x] Add semantic, native execution, cleanup, and zero-float regression tests
      for a packed Region element.

### Gate 38.8 — Aggregate descriptor and cleanup ABI

- [ ] Define the stable native ABI for a Region descriptor whose element type
      is a packed aggregate.
- [ ] Verify pass-by-value, pass-by-reference, and return lowering for packed
      Region descriptors without changing the public ownership contract.
- [ ] Generate exactly one cleanup bridge call for an owned packed Region on
      lexical drop, explicit close, failed publish, and failed cancellation.
- [ ] Verify that successful close prevents duplicate capability release and
      that failed operations preserve the owner.
- [ ] Add direct and nested generic execution tests for all cleanup paths.

### Gate 38.9 — Complete Region scale evidence

- [ ] Record compiler peak RSS, object size, symbol size, build time, and
      execution status for primitive and packed Region element profiles.
- [ ] Run logical capacities from MiB through TiB with a bounded resident
      window for both supported element categories.
- [ ] Verify that logical capacity does not change resident allocation,
      executable layout, or descriptor stride.
- [ ] Record exact compiler revision, commands, host profile, and evidence
      boundaries for every scale result.
- [ ] Keep unsupported aggregate forms and incomplete element types as explicit
      negative tests; do not silently convert them to inline arrays.

## Evidence — 2026-10-07

- Compiler source: `1f55d0d` (`main` baseline).
- Focused commands: `cargo test --test std_region_native --test
  std_region_scale -- --test-threads=1` and `cargo test --test
  std_region_scale -- --nocapture --test-threads=1`.
- Focused result: 8 Region tests passed. The current scale run reported
  `resident_bytes=1` and `executable_bytes=5775568` for 1 MiB, 1 GiB, and 1
  TiB logical lengths; build times were 100 ms, 64 ms, and 72 ms, and run
  times were 589 us, 587 us, and 653 us on the host.
- Full command: `cargo fmt --all -- --check`, `cargo check --all-targets
  --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --all-targets --all-features -- --test-threads=1`,
  `scripts/check_source_limits.sh`, and `git diff --check`.
- Full result: all commands passed; Region native tests reported no floating-
  point instructions.
- The public guide now describes only generic `Region[T]` usage, ownership,
  bounds, generation, cleanup, and the distinction from inline arrays.
- The large-capacity evidence does not claim that arbitrary inline aggregates
  scale without compiler materialization. That remains outside the completed
  Region contract and requires separate compiler work.
- Gate 38.5 remains open only for an independently reproducible compiler RSS,
  object-size, symbol-size, and build-time record. The logical-capacity and
  bounded-resident-window behavior itself is covered by the current test
  output above.
- The current implementation boundary is explicit: primitive Region elements
  and packed aggregate elements are covered by native evidence; descriptor ABI
  and complete scale evidence remain in Gates 38.8–38.9.

## Definition of done

Phase 38 is complete when `Region[T]` is a documented, tested, ownership-safe,
bounded-window abstraction for every supported sized element category,
including packed aggregates; large logical capacities compile and execute
without per-element compiler materialization; and all public behavior has
reproducible semantic, native, cleanup, and scale evidence.
