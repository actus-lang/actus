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

- [ ] Update parser, formatter, semantic tokens, hover, and completion data.
- [ ] Update the language guide and standard conformance fixtures.
- [ ] Add diagnostics to the coding guide.
- [ ] Run strict check, native build, executable tests, and ownership audits.
- [ ] Record compatibility impact and close this phase only with all evidence.

## Completion criteria

Phase 32 closes when explicit value reuse is visible in source, unsupported
copying remains rejected, fixed-slot selection is bounded and layout-safe, and
all semantic, native, ownership, and tooling gates pass.
