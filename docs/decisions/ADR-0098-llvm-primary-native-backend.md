# ADR-0098: LLVM as the primary native backend

- **Status:** Proposed
- **Date:** 2026-11-01
- **Scope:** native backend selection, compiler distribution, target support,
  backend parity, CI, and artifact compatibility

## Context

Actus currently has two native emitters. The backend-neutral semantic and
lowering contracts are shared, while machine emission, object generation, and
target-specific integration remain backend-owned. The default compiler path
currently selects Cranelift and LLVM is explicitly enabled as an alternative.

The project needs one clearly defined primary native backend so that default
builds, release artifacts, target support, diagnostics, and performance
evidence have one authoritative implementation. Making a backend primary must
not silently change Actus semantics or use the secondary backend to hide an
unsupported operation.

## Decision

LLVM will become Actus's primary native backend after the implementation and
acceptance work in the corresponding roadmap is complete.

The migration has these properties:

1. The default compiler configuration selects LLVM once the LLVM acceptance
   criteria are complete.
2. Cranelift remains available as an explicit secondary backend while its
   compatibility value is demonstrated by the backend-parity suite.
3. Backend selection remains explicit and typed. If the selected backend is
   unavailable for the requested build, target, or runtime profile, the
   compiler fails with a diagnostic; it never silently falls back to the other
   backend.
4. Parser, semantic analysis, ownership checking, generic specialization,
   shared lowering, target contracts, runtime contracts, and serialization
   formats remain backend-neutral.
5. LLVM-specific types, handles, optimization settings, native bindings, and
   object emission stay inside the LLVM backend boundary.
6. Cranelift-specific behavior stays inside the Cranelift boundary. Keeping
   Cranelift available does not permit backend-specific branches in semantic or
   shared lowering code.
7. Existing Actus source semantics, ownership roles, cleanup rules, public
   APIs, ABI contracts, and serialized layouts do not change as a consequence
   of backend selection.

## Required implementation contract

The migration is accepted only when all of the following are true:

- LLVM-only builds compile without the Cranelift feature.
- The default build selects LLVM and records that selection in artifact
  metadata.
- Explicit Cranelift builds remain reproducible when the secondary backend is
  supported for the target.
- Shared semantic and negative fixtures produce equivalent results under both
  backends wherever the target contract permits both.
- LLVM verifies every emitted module before object emission and reports
  source-spanned unsupported-operation diagnostics at the shared boundary.
- LLVM and Cranelift use the same checked scalar, aggregate, enum, ownership,
  cleanup, runtime-bridge, and external-ABI contracts.
- Hosted and freestanding target support is claimed only with matching object,
  link, execution, or target-specific evidence.
- The compiler never introduces implicit allocation, floating-point state into
  an integer-only contract, hidden I/O, or raw pointers in serialized state.
- Backend changes invalidate incompatible generated artifacts and do not reuse
  objects emitted by another backend.

## Compatibility and migration policy

This ADR does not remove Cranelift. During the migration, users can select it
explicitly where its target contract is supported. A later decision may retire
it only after a separate compatibility review, published evidence, and an
explicit deprecation path.

Changing the default backend is a build and artifact compatibility change. The
compiler must therefore:

- document the new default and the explicit Cranelift override;
- include the selected backend in reproducible artifact metadata;
- invalidate stale objects when the backend changes;
- keep source-level behavior independent of the selected emitter; and
- provide a rollback configuration that selects Cranelift without changing
  source code or serialized data.

## Alternatives considered

### Keep Cranelift as the default indefinitely

Rejected as the long-term policy because it leaves the compiler without one
authoritative backend for the full target and ABI roadmap.

### Remove Cranelift immediately

Rejected because it would discard an existing compatibility and parity oracle
before LLVM has completed target, runtime, ABI, and regression acceptance.

### Silently fall back between backends

Rejected because fallback would conceal unavailable target support and could
change object, ABI, or runtime behavior without a source-level indication.

### Duplicate semantic implementations in each backend

Rejected because it would allow ownership, cleanup, layout, and failure
semantics to diverge. Shared lowering remains the single semantic boundary.

## Consequences

Positive consequences:

- one documented default native path;
- a stable basis for target and ABI support work;
- explicit separation between semantic correctness and machine emission;
- continued Cranelift parity coverage during the migration; and
- clearer release, CI, and support statements.

Costs and risks:

- LLVM toolchain availability becomes part of the default development and
  release contract;
- compile time, binary size, cache behavior, and installation requirements
  must be measured rather than assumed;
- incomplete LLVM operations must be implemented or rejected explicitly before
  the default changes; and
- cross-target acceptance requires more than a successful host build.

## Evidence required before this ADR is accepted

- LLVM-only, default-LLVM, and explicit-Cranelift CI configurations;
- complete parser, semantic, shared-lowering, runtime, FFI, standard-library,
  and native regression results;
- target-specific object/link/execute evidence for every claimed target;
- deterministic artifact and symbol-manifest comparisons;
- zero-float and ownership/cleanup evidence where those contracts apply;
- measured compile-time, artifact-size, and cache impact; and
- a documented rollback procedure tested from a clean checkout.

