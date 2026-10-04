# Phase 32: Value Reuse and Fixed-Slot Selection

**Status:** Planned

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

- [ ] Define the bounded fixed-slot accessor or equivalent language form.
- [ ] Preserve element type, slot count, layout, and alignment metadata.
- [ ] Add accepted indexed-selection fixtures.
- [ ] Add rejected out-of-range and invalid-role fixtures.
- [ ] Verify direct and computed indexes produce identical bounds behavior.

### Gate 32.4 — Native lowering

- [ ] Lower eligible scalar reuse without identity arithmetic.
- [ ] Lower fixed-slot selection to deterministic native code.
- [ ] Verify aggregate stride and packed field offsets are unchanged.
- [ ] Verify no raw pointer escapes are introduced.
- [ ] Add native executable tests for repeated use and slot selection.

### Gate 32.5 — Tooling and conformance

- [ ] Update parser, formatter, semantic tokens, hover, and completion data.
- [ ] Update the language guide and standard conformance fixtures.
- [ ] Add diagnostics to the coding guide.
- [ ] Run strict check, native build, executable tests, and ownership audits.
- [ ] Record compatibility impact and close this phase only with all evidence.

## Completion criteria

Phase 32 closes when explicit value reuse is visible in source, unsupported
copying remains rejected, fixed-slot selection is bounded and layout-safe, and
all semantic, native, ownership, and tooling gates pass.
