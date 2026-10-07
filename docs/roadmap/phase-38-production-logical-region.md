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

- [ ] Document `Region[T]`, `RegionError`, and every public Region operation in
      the language guide.
- [ ] Document the difference between inline arrays and runtime-backed regions.
- [ ] Document `dat`, `erg`, `abs`, and `ins` behavior for every operation.
- [ ] Document bounds, stale-generation, capability, buffer-size, and backend
      failures without exposing native status values.
- [ ] Keep the guide free of downstream application and device terminology.

### Gate 38.2 — Semantic validation

- [ ] Accept sized primitive, array, pack, and struct element types.
- [ ] Reject unsized, incomplete, malformed, and multi-parameter Region types.
- [ ] Reject invalid logical lengths, windows, strides, alignments, and target
      address-width combinations before native lowering.
- [ ] Preserve the Region type identity through generic calls and facade
      imports.
- [ ] Add accepted and rejected semantic tests for every public constraint.

### Gate 38.3 — Native ABI and cleanup

- [ ] Keep the Region descriptor layout fixed-width, pointer-free, and stable.
- [ ] Preserve the descriptor contract through direct and nested generic calls.
- [ ] Generate exactly one cleanup action for every owned Region.
- [ ] Release a capability exactly once after successful close or lexical drop.
- [ ] Preserve the owner after failed close, publish, or cancellation.
- [ ] Add native execution tests for success, failure, nested calls, and early
      return cleanup.

### Gate 38.4 — Window lifecycle

- [ ] Validate open, read, write, publish, cancel, and close as one lifecycle.
- [ ] Reject stale generations immediately and prevent generation wraparound.
- [ ] Prevent incompatible publication, eviction, or reuse while a view is
      live.
- [ ] Keep dirty resident bytes usable after failed publication.
- [ ] Prove that cancellation restores the last published resident bytes.
- [ ] Keep all access explicit and free of implicit filesystem operations.

### Gate 38.5 — Large logical capacity evidence

- [ ] Check and build Region sources with logical lengths of 1 MiB, 1 GiB, and
      1 TiB without materializing one object per logical element.
- [ ] Record compiler RSS, object size, symbol size, and build time.
- [ ] Execute checked reads and writes within a bounded resident window.
- [ ] Prove that out-of-window and overflowed access fails deterministically.
- [ ] Prove that resident memory remains bounded as logical length increases.

### Gate 38.6 — Quality and release acceptance

- [ ] Run formatter, source-limit, compiler, semantic, native, and documentation
      checks.
- [ ] Run the full test suite with no filtered failures.
- [ ] Verify no floating-point instructions are introduced by Region lowering.
- [ ] Verify no reverse pipeline dependency or backend type leaks into frontend
      or standard-library contracts.
- [ ] Update the language guide and ADR evidence with only implemented generic
      behavior.
- [ ] Record exact compiler revision and reproducible commands.

## Definition of done

Phase 38 is complete when `Region[T]` is a documented, tested, ownership-safe,
bounded-window abstraction; large logical capacities compile and execute
without per-element compiler materialization; and all public behavior has
reproducible semantic and native evidence.
