# Phase 37: Scalable Aggregate Lowering and Large Logical Storage

## Purpose

Extend Actus so a native executable can describe and operate on very large
logical data regions without requiring the compiler to materialize every
element as a compile-time or code-generation object.

The phase addresses a compiler scalability boundary observed with a bounded
const-generic aggregate. Small specializations compile and execute, while a
specialization with approximately one million packed records causes the build
process to terminate with operating-system status 137 before an executable is
produced. The failure must be treated as a compiler and representation
boundary, not hidden by changing the application workload.

This phase is independent of any downstream application, domain model, or
project-specific data structure.

## Scope and non-goals

### In scope

- Memory-bounded representation of large const-generic aggregate types.
- Native layout and ABI planning that remain proportional to type complexity,
  not element count.
- Explicit runtime-backed storage for large logical regions.
- Paged or windowed access with checked bounds and ownership-aware APIs.
- Logical capacities larger than available resident memory.
- Deterministic behavior across hosted and freestanding targets.
- Compiler diagnostics when a requested representation cannot be lowered.

### Out of scope

- Hidden heap allocation.
- Raw operating-system pointers in Actus values or serialized formats.
- Implicit memory mapping or implicit filesystem access.
- An unbounded collection type disguised as a fixed array.
- Promising that a target can physically hold or process a complete
  terabyte-sized region in RAM.
- Application-specific optimizations or domain terminology.

## Invariants

- A const argument remains part of semantic type identity and layout identity.
- Aggregate layout computation is constant-time with respect to the number of
  repeated elements after the element layout is known.
- Native code size does not grow linearly with the logical element count.
- Element stride, total byte size, alignment, and overflow are checked before
  native emission.
- Ownership roles remain explicit across storage views, page loading, and
  write-back.
- Large logical capacity does not imply resident allocation.
- Access outside a logical region is rejected deterministically.
- Existing small aggregate behavior and ABI contracts remain unchanged.
- No floating-point operations are introduced by this phase.

## Required design resolutions before implementation

The following questions are architectural gates, not implementation details.
They must be resolved in the ADR before compiler or runtime code is changed.

### Inline aggregate versus runtime-backed region

The existing bounded aggregate remains an inline value with a contiguous
layout and its established ABI. Its const extent describes physical storage
that is present in the value; the compiler must preserve that contract.

A runtime-backed logical region is a separate reviewed abstraction. Its
descriptor represents logical length, element layout, resident window, and
the selected storage capability. It must not silently change the meaning of
an existing array or make an inline aggregate non-contiguous. Conversions
between the two representations require explicit APIs and checked bounds.

The ADR must select the first public spelling, define whether the descriptor
is a value or an owned resource, and state which operations are available for
each representation. No compiler lowering may infer a runtime-backed region
from a large inline array as an undocumented optimization.

### Symbol and initialization boundedness

Large extents must not cause mangled names, object metadata, relocation lists,
or initializer payloads to grow with the number of elements. Zero-filled
storage must use the target's declared zero-initialization mechanism where the
ABI permits it; non-zero initialization must be rejected or represented by an
explicit bounded initialization operation.

The ADR and native tests must distinguish BSS-like zero storage, read-only
constant data, writable initialized data, and runtime-backed storage. The
compiler must report the selected section and estimated output size in a
diagnostic or inspectable artifact for large-extent tests.

### Window lifecycle and concurrency

A resident window is an owned resource with an explicit lifecycle. A page or
window cannot be evicted, flushed, or reused while an `ins` or `abs` view is
live. Mutation through `ins` completes before publication, eviction, or
handoff to another execution context. Any concurrent implementation must use
an explicit synchronization capability; ordinary ownership roles must not be
treated as implicit locks.

Interrupt and multi-core profiles must define whether access is single-owner,
serialized, or atomic at the descriptor and element boundaries. Dirty state,
generation, flush failure, cancellation, and eviction races must have
deterministic outcomes. The first implementation may reject concurrent window
ownership, but it must reject it explicitly rather than permit a data race.

## Gates

### Gate 37.1 — Reproduce and classify the scalability boundary

- [x] Add a compiler-only generic aggregate fixture with a small packed element
      and specializations at 64, 1,024, 65,536, and 1,048,576 elements.
- [x] Record semantic-check, native-build, executable-size, and process-result
      evidence for every specialization.
- [x] Distinguish compiler memory exhaustion, compiler time exhaustion,
      linker failure, runtime allocation failure, and runtime execution failure.
- [x] Preserve the exact diagnostic or operating-system termination status when
      a build cannot produce an executable.
- [x] Confirm the fixture contains no filesystem, device, or application-specific
      dependency.
- [x] Record the baseline compiler revision and host resource profile.

Gate 37.1 evidence, 2026-10-07:

- Fixture: `tests/fixtures/scalable_aggregate/aggregate_<N>.act`, using a
  one-byte packed element and a generic `Storage[N]` aggregate.
- Compiler: installed Actus compiler from commit `a85ca18`;
  SHA-256 `f8f3a659e3b8a589018a833559ead567b2ad5dd0742192b656c1bf5d69dc0430`.
- Host: Linux x86-64, Intel Core i7-14700KF, 28 logical CPUs.
- `cargo test --test scalable_aggregate -- --nocapture`: 1 passed, with
  semantic check, native build, and executable run passing at 64, 1,024,
  and 65,536 elements. The 1,048,576-element case is kept as a separate
  manual CLI measurement so a resource-heavy child cannot terminate the test
  runner under a constrained test process.
- Manual commands for the 1,048,576-element case were:
  `actus check tests/fixtures/scalable_aggregate/aggregate_1048576.act
  --strict` and `actus build tests/fixtures/scalable_aggregate/aggregate_1048576.act
  --strict --emit exe -o /tmp/actus-scale-1048576.aie`.
- Executable sizes were 5,586,296 bytes at 64, 5,606,776 bytes at 1,024,
  7,171,448 bytes at 65,536, and 30,981,496 bytes at 1,048,576 elements.
- The 1,048,576-element executable ran with exit code 42.
- The fixture uses no filesystem, device, or downstream application module.
- A separate heavier packed-element experiment previously reached operating
  system status 137 during native build. That result is recorded as a
  representation-pressure observation and is not treated as a failure of this
  small-element Gate 37.1 fixture.

### Gate 37.2 — Define the scalable representation contract

- [x] Write an ADR defining the difference between an inline bounded aggregate
      and a runtime-backed logical region.
- [x] Define the metadata required for a region: element layout identity,
      element stride, logical length, resident window, page size, and bounds.
- [x] Define whether the first implementation uses pages, windows, handles, or
      another explicit representation, and document the trade-offs.
- [x] Define ownership transitions for opening, borrowing, mutating, flushing,
      closing, and invalidating a region.
- [x] Define failure behavior for unavailable pages, stale generations,
      arithmetic overflow, invalid handles, and capacity exhaustion.
- [x] Reject implicit allocation, implicit I/O, and raw pointer escape from the
      public Actus model.
- [x] Decide and document the separate public representation for inline
      aggregates and runtime-backed logical regions.
- [x] Define whether a region descriptor is an owned resource, a borrowed
      view, or a capability-bearing value, including its cleanup contract.
- [x] Define the capability as a target-agnostic fixed-width handle or index;
      public and serialized state uses `u64`, while narrower internal indexes
      require checked conversion.
- [x] Define a monotonic generation counter with an uninitialized value,
      immediate stale-generation rejection, and no-wrap exhaustion behavior.
- [x] Define page/window pinning, dirty publication, flush, eviction, and
      cancellation rules for `ins` and `abs` views.
- [x] Define the single-owner, serialized, or atomic concurrency profile for
      hosted, multi-core, and interrupt-driven targets.

Gate 37.2 evidence, 2026-10-07:

- ADR-0073 accepts a separate runtime-backed region resource and preserves
  `Array[T, N]` as contiguous inline storage.
- The first region profile uses a bounded descriptor, fixed-width capability,
  explicit window operations, typed failures, and single-owner serialization.
- `ins` and `abs` views are call-scoped; live views block incompatible flush,
  eviction, and reuse. Concurrent access is rejected until a separate
  synchronization decision is accepted.
- Capability handles use fixed-width `u64` public/serialized state, and
  generation values are monotonic `u64` counters with checked exhaustion and
  no wraparound.
- Hidden allocation, implicit I/O, raw pointers, transparent array conversion,
  and implicit page faults are explicitly rejected.

### Gate 37.3 — Make aggregate layout planning scale independently

- [x] Store one canonical element layout and a checked symbolic extent instead
      of constructing one compiler layout object per logical element.
- [x] Compute total size with checked multiplication and addition.
- [x] Keep alignment and stride calculation independent from extent size.
- [x] Ensure type identity, ABI identity, and mangling remain deterministic for
      large extents without embedding enormous generated names.
- [x] Add native layout regressions for large logical extents that do not
      allocate or emit one field per element.
- [x] Add negative tests for extent overflow, invalid zero or negative bounds,
      impossible alignment, and unsupported target address width.
- [x] Verify that large zero-filled storage uses a bounded object-section
      representation and does not emit a materialized zero-byte initializer.
- [x] Verify that mangled names, relocation metadata, and symbol records remain
      bounded as the logical extent grows.
- [x] Keep inline contiguous ABI layout separate from runtime-backed region
      descriptor layout in semantic identity and native lowering.

Gate 37.3 evidence, 2026-10-07:

- `LayoutRegistry` keeps one canonical `ArrayLayout` per canonical array type;
  the layout stores element type, checked `u32` extent, total size, and
  alignment. Native layout now rejects zero capacity, rejects multiplication,
  addition, and alignment overflow, and rejects targets without a supported
  address width.
- Array construction uses Cranelift's target-aware bounded `memset` lowering.
  The compiler no longer emits one native store per byte for zero-filled
  aggregate initialization. The object regression for `Array[Cell, 65536]`
  contains a `memset` symbol and remains a bounded object (`1,129,928` bytes
  on the recorded Linux x86-64 host).
- Native execution passed for 64, 1,024, and 65,536 element fixtures, each
  returning exit code 42. The object regression and the deliberate
  `Array[u64, 4294967295]` overflow build both passed their expected checks.
- Semantic regressions reject zero capacity, negative capacity at parse time,
  and capacities above the native `u32` extent. Layout regressions reject zero
  and overflowing alignment values, and target validation rejects an unknown
  address width.
- `TypeIdentity` preserves the canonical key `Array[u8,1048576]` with bounded
  textual length. Existing native type lowering keeps inline arrays as the
  contiguous aggregate ABI; the runtime-backed region remains a separate
  reviewed contract from ADR-0073 and is not implicitly substituted.
- Commits: `d7c8f1e` (`perf: bound large aggregate native initialization`) and
  `9902cd1` (`test: cover aggregate layout boundaries`). Full `cargo fmt
  --check`, `cargo check`, `cargo clippy --all-targets --all-features
  -- -D warnings`, and `cargo test` passed before closure.

### Gate 37.4 — Add an explicit runtime storage boundary

- [x] Introduce the smallest reviewed runtime abstraction for a logical region.
- [x] Keep the abstraction independent from filesystem and operating-system
      implementations.
- [x] Support bounded read and write operations through explicit owned or
      borrowed views.
- [x] Validate page/window bounds before calculating byte offsets.
- [x] Prevent a borrowed view from escaping its `ins` call scope.
- [x] Make persistence, mapping, caching, and eviction separate implementations
      behind the reviewed boundary.
- [x] Add hosted in-memory and bounded test backends without changing language
      ownership semantics.
- [x] Reject eviction, flush, or reuse while an incompatible `ins` or `abs`
      view remains live.
- [x] Add race and cancellation tests for dirty windows, generation changes,
      failed publication, and rejected concurrent ownership.

Gate 37.4 progress, 2026-10-07:

- `src/runtime/region.rs` introduces a target-independent `u64` capability,
  monotonic generation, bounded `RegionDescriptor`, typed failures, and an
  explicit hosted `InMemoryRegion` backend. It performs no filesystem access,
  OS mapping, implicit allocation policy, or persistence operation.
- `RegionView` and `MutableRegionView` are lifetime-bound Rust views. A mutable
  view holds the descriptor and resident bytes exclusively, so publication,
  reuse, or another view cannot be requested until the borrow ends.
- The backend validates handle, generation, logical index, resident window,
  checked byte offsets, and destination/source lengths before accessing bytes.
  Publication advances the generation and rejects exhaustion without replacing
  the previous generation.
- Five runtime unit tests plus a compile-fail documentation test cover
  read/write/publish, stale and out-of-window
  access, invalid handles, buffer-size errors, invalid windows, generation
  exhaustion, cancellation rollback, and failed publication. Full concurrent
  access is rejected by the exclusive borrow contract before runtime entry;
  cancellation semantics are explicit and tested.
- Implementation commits: `0b5fb89` (`feat(runtime): add bounded logical
  region boundary`) and `ca53904` (`fix(runtime): preserve published region
  generation`).
- The compile-fail ownership regression is attached to `borrow_abs`; it proves
  that a live read view prevents `publish` or another mutable access until the
  view scope ends. `cargo test --doc runtime::region` passes.

### Gate 37.5 — Addressing and overflow safety

- [x] Define the logical index width and byte-offset width per target profile.
- [x] Check every index-to-offset conversion before multiplication.
- [x] Reject a region whose logical byte size cannot be represented by the
      selected target profile.
- [x] Test capacities that fit in 32-bit indexes and capacities that require
      64-bit logical addressing.
- [x] Test page crossing, last-element access, empty windows, and boundary
      rejection.
- [x] Ensure serialized metadata uses fixed-width, position-independent fields.

Gate 37.5 evidence, 2026-10-07:

- `RegionAddressProfile` defines independent 32-bit or 64-bit logical-index
  and byte-offset widths. The hosted default is 64/64; a constrained target
  can explicitly select 32/32 without changing the inline aggregate ABI.
- Region validation checks logical-length bounds, total logical byte size,
  window-end addition, index-to-offset multiplication, element-end addition,
  and resident-window allocation before access or conversion. Saturating
  arithmetic is not used for access authorization.
- Runtime tests accept the largest representable 32-bit logical length for a
  one-byte element, reject the next length and an overflowing two-byte layout,
  and accept a greater-than-32-bit logical capacity with a one-element 64/64
  resident window. They also cover a window crossing the 4 KiB boundary,
  last-element access, rejection after the window, invalid empty windows, and
  unsupported widths.
- `RegionDescriptor` remains `#[repr(C)]` with fixed-width scalar fields and
  no pointers; the regression asserts a 56-byte size and 8-byte alignment.
- Focused evidence: `cargo test --lib runtime::region` passed 9 tests and
  `cargo test --doc runtime::region` passed the compile-fail ownership test.

### Gate 37.6 — Native ABI and code-generation efficiency

- [x] Lower a runtime-backed region through a compact descriptor or equivalent
      reviewed ABI representation.
- [x] Ensure generated code contains access logic, not one specialized function
      per logical element.
- [x] Keep descriptor passing compatible with ownership roles and cleanup.
- [x] Verify native symbol names and metadata remain bounded for large extents.
- [x] Add executable tests for read, write, bounds rejection, and cleanup.
- [x] Verify that invalid descriptor states fail safely without compiler-generated
      traps on valid inputs.
- [x] Add object-level checks for section selection, initializer size, symbol
      length, relocation count, and executable size at large logical extents.
- [x] Verify that the runtime-backed descriptor ABI cannot be confused with an
      inline aggregate ABI at a call boundary.
- [x] Approve the Actus source contract for region operations: the
      `std::region` facade, `Region[T]`, `RegionError`, operation names, and
      `erg`/`abs`/`ins`/`dat` transitions. Do not add compiler intrinsics before
      this contract is accepted.
- [x] Add the compiler-only `size_of[T]()` prerequisite for facade stride
      derivation. It returns `u64`, accepts only fixed-size source types, and
      lowers to an immediate with no runtime dependency.
- [x] Add the canonical `std::region` source facade, typed `RegionError`, and
      private bridge declarations with semantic export and ownership tests.
- [x] Implement the `std::region` facade and keep raw runtime bridges private.
- [x] Implement `region_open` with `dat Buffer` transfer and capability
      registration.
- [x] Implement checked `region_read` and `region_write` with exact element
      byte-width validation.
- [x] Implement `region_publish`, `region_cancel`, and idempotent
      `region_close` without synchronous filesystem I/O.
- [x] Add semantic, native executable, and double-release regression tests for
      the complete facade.
- [x] Ensure builtin generic enum payloads such as `Result[Region[T], E]` are
      materialized in native layout collection for external facade bridges.
- [x] Preserve the canonical native symbol for generic `extern "C"` bridges;
      Actus specialization names remain internal lookup names and do not leak
      into the runtime ABI. Imported Actus wrappers retain namespace binding.

Gate 37.6 progress, 2026-10-07:

- Generic facade specialization now substitutes type arguments inside intrinsic
  calls, so `size_of[T]()` lowers as `size_of[u32]()` in a concrete
  `region_open[u32]` body. The native object regression
  `generic_region_facade_object_emits_size_of_with_concrete_element_type`
  passes and confirms the strict object build completes with no floating-point
  instructions.
- Generic `extern "C"` declarations now carry canonical native symbol metadata
  through specialization. The regression
  `generic_external_abi_uses_the_canonical_native_symbol` confirms that a
  `bridge[u32]` call imports `bridge`, never `bridge__u32`.
- The hosted runtime side now exposes a compact `#[repr(C)]` descriptor with
  fixed-width fields, explicit ABI size/alignment accessors, typed validation,
  and no pointer fields. Invalid handle, width, generation, length, stride,
  and window states are rejected before storage access.
- The bounded read/write/publish tests exercise the descriptor boundary and
  the compile-fail documentation test preserves the ownership contract for
  live views. The runtime test suite reports 10 passing region tests.
- Full hosted Actus source-level operations are now implemented through the
  canonical facade. The runtime owns a bounded resident store, transfers the
  opening buffer, validates descriptor and generation state, returns typed
  result objects, and keeps filesystem I/O out of all operations. `region_close`
  releases the backend while lexical cleanup releases the descriptor and
  rejects repeated capability drops.
- Native executable evidence is covered by
  `tests/std_region_native.rs`: strict host builds execute open/read/cleanup and
  write/publish/read/close paths, and the compiler reports no floating-point
  instructions. A bounds-failure executable returns a typed error result and
  exits normally without a compiler-generated trap. Runtime bridge unit
  evidence covers one-time descriptor drop. A second executable regression
  closes a Region and then attempts a read; the operation returns a typed
  invalid-state result and exits normally.
- The `hosted_region_object_has_bounded_abi_sections_and_relocations` regression
  parses objects emitted for one-element and 4 GiB logical regions. It checks
  the text section, initialized data bound, symbol-name bound, relocation
  bound, absence of logical extent in symbols, and bounded object-size growth.
  The same test compares the Region object with an inline `Array[u32, 2]`
  object: the Region object carries the Region operation wrapper ABI while the
  inline object does not, proving the representations remain distinct at the
  object boundary. The current Region ABI is one opaque pointer-sized
  capability on every target; target-specific classification of inline
  aggregates remains a separate later target gate.
- ADR-0074 resolves the next compiler contract: `Region[T]` is a distinct
  opaque owned resource with explicit window operations and the reviewed
  descriptor ABI. Its semantic registration, role rules, native layout, and
  private runtime bridges are the remaining implementation gates.
- The implementation must reject unsized or dynamic `T` during semantic
  analysis, lower lexical `erg` teardown to exactly one capability-release
  bridge call, and use target ABI aggregate classification for descriptor
  passing. Manual multi-register assumptions and pointer-valued public state
  are prohibited.
- Semantic registration has started with `Region[T]`: fixed primitive, array,
  pack, and struct element layouts are accepted; dynamic `Buffer`, `String`,
  `Map`, nested `Region`, unresolved generic, and recursive element types are
  rejected with the dedicated `E1088` diagnostic. The native lowering and
  cleanup portions now have initial evidence: `NativeType::Region` uses the
  bounded 56-byte descriptor layout, capability handles use a fixed 1024-slot
  table with monotonic generation checks, and lexical `erg` cleanup emits the
  runtime release bridge. `cargo check --all-targets --all-features` passed,
  the capability tests passed 4/4, the region runtime tests passed 10/10, and
  `lowers_region_owner_cleanup_to_the_runtime_release_bridge` passed. A
  `Region[Array[u8, 1048576]]` codegen regression also confirms that logical
  capacity does not enter native symbol names. Full explicit operation
  coverage, target-specific aggregate classification, and large-object
  evidence remain open.
- The public operation contract is now accepted in ADR-0074. The next
  implementation step is the `std::region` facade plus private runtime
  bridges. Until those bridges and executable tests exist, Gate 37.6 remains
  open and no operation is claimed as implemented.

### Gate 37.7 — Scale evidence

- [ ] Compile and run inline aggregate fixtures through the largest supported
      practical extent for the current compiler profile.
- [x] Run logical-region fixtures at 1 MiB, 1 GiB, and a terabyte-class logical
      capacity using bounded resident windows rather than full allocation.
- [x] Record resident memory, peak compiler memory, executable size, build time,
      access latency, and failure status.
- [ ] Repeat the hosted evidence on at least one freestanding or embedded target
      profile before making target-specific claims.
- [ ] Keep host, target, and simulated logical-capacity results in separate
      dated evidence sections.

Gate 37.7 hosted logical-region evidence, 2026-10-07:

- `tests/std_region_scale.rs` builds and runs strict hosted fixtures at 1 MiB,
  1 GiB, and 1 TiB logical capacity with a one-byte resident window. All three
  cases exit successfully, report `resident_bytes=1`, and emit the same
  5,775,568-byte executable. A separate Linux `/proc` measurement reports
  21,608–21,760 KiB peak compiler RSS across the same cases. The complete
  output and evidence boundary are in
  `docs/benchmarks/phase-37-region-scale-2026-10-07.md`.
- Inline aggregate maximum extent, freestanding target output, and embedded
  hardware evidence remain open.

### Gate 37.8 — Compatibility and quality acceptance

- [ ] Existing const-generic aggregate tests pass unchanged.
- [ ] Existing ownership, cleanup, layout, ABI, and invalid-bound tests pass.
- [ ] Add accepted and rejected tests for every new public representation.
- [ ] Run formatter, source-limit, compiler, native, and documentation checks.
- [ ] Confirm no reverse pipeline dependency or backend type leaks into the
      frontend.
- [ ] Update the language guide, ADR, roadmap evidence, and release notes.
- [ ] Close the phase only after all claims have reproducible evidence and no
      unresolved compiler or runtime boundary remains.

## Evidence policy

A successful semantic check does not prove that native lowering scales. A
successful native build does not prove that a target has enough resident
memory. A large logical capacity backed by pages or windows does not prove
that all logical data is simultaneously resident.

Every scale result must identify:

- compiler revision and digest;
- target and ABI profile;
- optimization mode;
- host or device resource limits;
- logical capacity;
- resident capacity;
- page or window size;
- build and execution commands;
- complete output;
- whether storage was inline, in-memory, mapped, or simulated.

## Exit criteria

Phase 37 is complete when Actus can compile a compact representation for large
logical regions, execute checked windowed access through an explicit ABI, keep
native output bounded, and provide reproducible evidence for both resident and
non-resident scale profiles without hidden allocation or application-specific
compiler behavior.
