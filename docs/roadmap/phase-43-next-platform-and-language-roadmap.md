# Phase 43 — Actus Platform and Language Completion Roadmap

## Status

Planned. This document is an umbrella roadmap for the next group of Actus
work. It records the remaining directions after the completed Region, Wire,
aggregate lowering, and language handbook work.

The gates in this file are planning gates. They do not claim that the work is
implemented. Before implementation begins, each substantial gate must receive
its own focused execution phase or child roadmap with source, test, and
documentation acceptance criteria.

## Purpose

Keep the Actus roadmap coherent after the completion of the current runtime
and standard-library foundations. The roadmap covers six independent areas:

1. roadmap and evidence reconciliation;
2. target and embedded backend support;
3. complete String and UTF-8 facilities;
4. language tooling and developer workflow;
5. enterprise compiler capabilities;
6. future language capabilities and self-hosting.

These areas must remain separate in implementation. A compiler backend change
must not be mixed with standard-library API work, and future language features
must not be presented as completed platform guarantees.

## Architectural boundaries

- Actus remains a systems programming language and compiler project.
- Standard-library work must preserve explicit ownership, bounded storage,
  deterministic failure values, and visible runtime boundaries.
- Hosted and freestanding targets must be described by explicit capability
  profiles.
- Embedded support requires target-specific compiler, linker, runtime, and
  hardware evidence. Host tests alone do not prove embedded behavior.
- String and UTF-8 APIs must define byte units, scalar units, capacity,
  malformed-input behavior, ownership, and allocation behavior explicitly.
- Tooling must consume the same parser, semantic model, diagnostics, and
  module contracts as the compiler.
- Enterprise compiler features must preserve deterministic output and must not
  introduce hidden allocation, ownership shortcuts, or reverse pipeline
  dependencies.

## Gate 43.1 — Reconcile the roadmap and evidence index

- [ ] Inventory every open checkbox in historical, active, and future roadmap
      files.
- [ ] Classify each item as implemented, superseded, intentionally deferred,
      non-goal, or still requiring implementation.
- [ ] Reconcile Phase 37, Phase 39, Phase 40, Phase 41, and the Wire ADR with
      the merged source, tests, and documentation.
- [ ] Remove no historical record; update stale status and evidence sections
      so completed work is not represented as unfinished.
- [ ] Record the exact compiler revision, target profiles, test commands, and
      known limitations for every accepted completion claim.
- [ ] Produce one maintained index linking active gates to source and tests.

## Gate 43.2 — Target and embedded backend readiness

- [ ] Define supported hosted, freestanding, and embedded target classes.
- [ ] Define the target capability contract for pointer width, byte order,
      alignment, linker, runtime providers, and standard-library modules.
- [ ] Add the selected ARM Thumb backend to the native lowering pipeline only
      after the target contract and ABI are specified.
- [ ] Preserve the freestanding entry contract and reject unavailable hosted
      services before code generation.
- [ ] Add target-specific object, link, and no-host-runtime tests.
- [ ] Document the provider and linker inputs required for an embedded image.
- [ ] Flash and execute a complete generated image on an approved target only
      when reproducible hardware evidence is available.
- [ ] Record target compiler version, linker version, symbols, image layout,
      flash procedure, and observed runtime results.
- [ ] Keep hardware validation separate from host compiler and standard-library
      acceptance.

## Gate 43.3 — Complete String and UTF-8 facilities

- [ ] Audit every public String and UTF-8 declaration against its implementation
      and runtime provider contract.
- [ ] Define byte indexing and Unicode scalar indexing as separate operations.
- [ ] Add checked UTF-8 scalar decoding for one-, two-, three-, and four-byte
      sequences.
- [ ] Reject truncated sequences, invalid continuation bytes, surrogate code
      points, and out-of-range scalar values with typed failures.
- [ ] Define byte-exact equality and ordering for validated UTF-8 values.
- [ ] Define bounded prefix, suffix, search, and checked slicing operations.
- [ ] State whether every returned index is a byte offset or a scalar position.
- [ ] Add a caller-buffer-backed UTF-8 builder with explicit capacity and
      transactional failure behavior.
- [ ] Support append of validated UTF-8, borrowed String values, and bytes
      without hidden allocation.
- [ ] Define scalar-to-UTF-8 encoding and the supported ASCII-only case rules.
- [ ] Add exact-byte tests for empty values, embedded NUL bytes, ASCII,
      multibyte text, combining sequences, malformed input, and exhaustion.
- [ ] Verify deterministic cleanup, zero-float behavior, and hosted/freestanding
      compatibility for every public operation.
- [ ] Update the language handbook and standard-library guide from the accepted
      implementation contract.

## Gate 43.4 — Language tooling and developer workflow

- [ ] Make parser recovery boundaries and synchronization tokens explicit.
- [ ] Ensure diagnostics preserve stable codes, source spans, severity, and
      deterministic ordering across CLI and LSP consumers.
- [ ] Implement semantic tokens from the compiler semantic model.
- [ ] Implement completion from visible modules, declarations, roles, and
      ownership state.
- [ ] Implement code actions and quick fixes only where the compiler contract
      can describe a safe transformation.
- [ ] Implement inlay hints for types, roles, and ownership where the result is
      deterministic and useful.
- [ ] Add formatter, parser, LSP, diagnostics, and module-boundary regression
      tests for every tooling capability.
- [ ] Verify the offline documentation command and local developer workflow on
      supported host platforms.
- [ ] Package the tooling and document installation, versioning, and rollback.

## Gate 43.5 — Enterprise compiler platform

- [ ] Define stable source, dependency, configuration, target, and toolchain
      identities for cache invalidation.
- [ ] Bound cache storage, stale artifacts, cancellation, and failed-build
      cleanup.
- [ ] Prove clean-build equivalence for serial, incremental, and parallel
      compilation modes.
- [ ] Add deterministic parallel scheduling and race regression tests.
- [ ] Define target-independent source identity and line mapping contracts.
- [ ] Emit debug information without exposing backend-specific types through
      frontend or semantic modules.
- [ ] Add reproducible diagnostics and machine-readable reports for compiler
      latency, memory, cache behavior, and output identity.
- [ ] Profile the borrow checker and remove avoidable superlinear behavior only
      with semantic-equivalence tests.
- [ ] Verify optimized and debug builds preserve the same language semantics.
- [ ] Publish a release report containing benchmark conditions, deltas, target
      evidence, known limitations, and compatibility impact.

## Gate 43.6 — Future language capabilities and self-hosting

- [ ] Specify explicit views, zero-copy slices, and borrowed iterators before
      implementation.
- [ ] Specify ownership and cleanup for structs containing references.
- [ ] Define advanced reborrow chains and shared mutable access rules.
- [ ] Define task transfer, cancellation, scheduling, and failure ownership
      before introducing concurrency or asynchronous execution.
- [ ] Define closure capture ownership and cleanup before parser support.
- [ ] Create separate ADRs for macros, async tasks, concurrency, and self-hosting.
- [ ] Keep each future capability behind its own parser, semantic, codegen,
      runtime, tooling, and documentation acceptance gates.
- [ ] Treat self-hosting as a later major direction with an explicit bootstrap
      plan; it is not part of the current platform completion criteria.

## Completion contract for Phase 43

- [ ] Every accepted gate has implementation, tests, documentation, and
      reproducible evidence.
- [ ] No historical roadmap item is left ambiguous about whether it is done,
      deferred, or intentionally excluded.
- [ ] No target, performance, or hardware claim relies only on a host build.
- [ ] No public API, ABI, ownership rule, binary layout, or compiler behavior
      changes without an ADR and migration impact analysis.
- [ ] The next implementation phase is selected from the reconciled active
      work, with unrelated future capabilities remaining explicitly separate.
