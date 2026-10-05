# Phase 32: Value Reuse and Fixed-Slot Selection

**Status:** Complete

This phase implements [ADR-0059](../decisions/ADR-0059-value-reuse-and-fixed-slot-selection.md).
It improves source clarity for repeated read-only scalar use and bounded
selection across fixed-layout fields without weakening Actus ownership,
layout, bounds, or native safety guarantees.

## Scope

- explicit reuse for compiler-approved small scalar values;
- read-only scalar parameter contracts;
- bounded fixed-slot selection;
- semantic diagnostics and native lowering evidence;
- formatter, LSP, documentation, and conformance updates.

## Gates

### Gate 32.1 — Contract and type eligibility

- [x] Define the source syntax for explicit scalar reuse.
- [x] Define the compiler-approved eligible type set.
- [x] Reject reuse for buffers, resources, aggregates with cleanup, and
      unsupported user-defined types.
- [x] Preserve `erg`, `abs`, `dat`, and `ins` meaning at every call boundary.
- [x] Document diagnostics and migration guidance.

### Gate 32.2 — Semantic analysis

- [x] Add accepted fixtures for repeated scalar reuse.
- [x] Add rejected fixtures for implicit or unsupported copying.
- [x] Verify moved bindings remain rejected after an owning call.
- [x] Verify read-only scalar calls do not consume the caller binding.
- [x] Verify branch joins and cleanup state remain deterministic.

Evidence: `tests/semantic_intrinsics.rs` covers accepted repeated scalar use,
rejection of implicit and aggregate copying, use-after-move diagnostics after
an owning transfer, owner preservation across branch joins, and the existing
cleanup-aware semantic checks. The targeted semantic suite passes with
47 tests.

### Gate 32.3 — Fixed-slot selection

- [x] Define the bounded fixed-slot accessor or equivalent language form.
- [x] Preserve element type, slot count, layout, and alignment metadata.
- [x] Add accepted indexed-selection fixtures.
- [x] Add rejected out-of-range and invalid-role fixtures.
- [x] Verify direct and computed indexes produce identical bounds behavior.

Evidence: indexed access through `values[index]` is the bounded fixed-slot
form. Semantic coverage in `tests/semantic/arrays.rs`, `tests/semantic/packs.rs`,
and `tests/semantic/loans.rs` verifies element typing, declared capacity,
packed layout metadata, constant out-of-range rejection, invalid index types,
and ownership-role restrictions. Native coverage in `tests/arrays_cli.rs`
passes 72 tests, including dynamic bounds traps and contiguous indexed access.

### Gate 32.4 — Native lowering

- [x] Lower eligible scalar reuse without identity arithmetic.
- [x] Lower fixed-slot selection to deterministic native code.
- [x] Verify aggregate stride and packed field offsets are unchanged.
- [x] Verify no raw pointer escapes are introduced.
- [x] Add native executable tests for repeated use and slot selection.

Evidence: scalar `copy` lowers directly to its validated operand, while
indexed access uses the declared element size and native bounds checks. Native
coverage in `tests/arrays_cli.rs` includes explicit scalar reuse, direct and
computed fixed-slot selection, contiguous array access, packed field offsets,
aggregate stride, and deterministic repeated object generation. The compiler
architecture checks reject raw pointer escapes at the language boundary.

### Gate 32.5 — Tooling and conformance

- [x] Update parser, formatter, semantic tokens, hover, and completion data.
- [x] Update the language guide and standard conformance fixtures.
- [x] Add diagnostics to the coding guide.
- [x] Run strict check, native build, executable tests, and ownership audits.
- [x] Record compatibility impact and close this phase only with all evidence.

Evidence: parser and formatter fixtures cover `copy(value: abs value)` and
indexed selection. LSP coverage verifies completion, hover, signature help
catalogs, and semantic-token classification for the intrinsic; the existing
indexed access model remains exposed with type, capacity, layout, and bounds
facts. The accepted conformance fixture is
`tests/fixtures/phase32/value_reuse_and_fixed_slots.act`.

Acceptance results: `actus check --strict` accepts the fixture, strict native
build and execution succeed, parser 76/76, formatter 22/22, conformance 4/4,
LSP 82/82, strict CLI 9/9, semantic 192/192, native executable 73/73, and
architecture/ownership checks pass. The compatibility change permits a
module-scoped verb named `copy`; intrinsic dispatch is used only when no local
signature is present, preserving existing standard-library APIs.

### Gate 32.6 — Intrinsic and imported generic name resolution

- [x] Reproduce the failure where an imported generic `copy[R, W]` prevents
      scalar `copy(value: abs scalar)` from reaching intrinsic dispatch.
- [x] Preserve local module-scoped `copy` declarations as the higher-priority
      source-level declaration.
- [x] Route the one-argument scalar form to the validated intrinsic when only
      an imported generic `copy` is visible.
- [x] Keep the two-argument reader/writer form bound to the imported generic
      standard-library verb.

Evidence: `tests/applications.rs` covers both a direct imported `io` module
and a nested facade importing `io`. Both scalar intrinsic reuse and the
standard-library reader/writer copy path pass native execution.

### Gate 32.7 — Nested facade and ownership regression coverage

- [x] Add a nested-facade fixture combining an imported generic verb, a
      `case dat` payload, and an `ins` aggregate call.
- [x] Verify the fixture preserves concrete generic arguments after native
      specialization.
- [x] Verify no unresolved generic parameter reaches the second semantic pass.

Evidence: `nested_facade_keeps_scalar_copy_intrinsic_separate_from_imported_io_copy`
passes object emission, executable emission, and execution.

### Gate 32.8 — Native acceptance and compatibility

- [x] Pass strict check, object emission, executable emission, and execution
      for the regression fixture.
- [x] Pass the existing standard-library `io::copy` tests and value reuse tests.
- [x] Record the diagnostic and dispatch rule in the language and coding
      guides.
- [x] Close the phase only after the full compiler and twin-e evidence passes.

Gate 32.8 evidence so far: full `cargo test` passes, including 22 application
tests, 25 standard-IO tests, 73 native array tests, 192 semantic tests, and
the new nested-facade regression. The remaining twin-e command now reaches a
real source ownership diagnostic in `history_contains`; it no longer fails
with unresolved generic parameter `W`.

### Gate 32.9 — Native reanalysis preserves imported aggregate field types

- [x] Reproduce the post-specialization mismatch where an imported aggregate
      field typed as `u32` is observed as `Result[Int, IoError]`.
- [x] Preserve field and member types across the second semantic analysis.
- [x] Add a nested-facade regression using an imported aggregate and a generic
      call graph.
- [x] Pass twin-e context and generation native acceptance after the fix.

Gate 32.9 evidence: native reanalysis preserves imported aggregate field types;
the nested-facade generic regression passes; `twin-e` strict tests pass 16/16;
the context ambiguity benchmark reports `contexts=2 successors=2 isolated=1`;
the generation benchmark reports EOS completion and loop-guard behavior; and
all generated native IR reports no floating-point instructions.

## Completion criteria

Phase 32 is complete. Explicit value reuse is visible in source, unsupported
copying remains rejected, fixed-slot selection is bounded and layout-safe, the
intrinsic/imported-generic collision is resolved, imported aggregate field
types survive native reanalysis, and the compiler plus twin-e evidence passes.
