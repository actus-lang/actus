# Phase 22: Enterprise Compiler Platform

Phase 22 tracks enterprise capabilities that extend beyond the correctness and
Wire-readiness boundary of Phase 21. It exists so deferred work remains
visible, owned, and evidence-driven rather than being lost in an informal
backlog.

This phase follows the scope boundary in
[ADR-0053](../decisions/ADR-0053-production-language-capability-and-wire-readiness.md).
No item is complete from design text alone. Each gate requires implementation,
tests, documentation, and reproducible evidence appropriate to its boundary.

## Gate 22.0: Platform baseline and measurement

- [ ] Define supported host, freestanding, and future target classes.
- [ ] Establish compiler latency, memory, peak parallelism, cache-hit, and
      generated-artifact benchmarks from reproducible fixtures.
- [ ] Define compatibility, cache invalidation, debug-info, and diagnostic
      versioning contracts.
- [ ] Add machine-readable reports for every benchmark and platform gate.

## Gate 22.1: Parser recovery and diagnostics

- [ ] Define parser recovery boundaries and synchronization tokens.
- [ ] Collect multiple independent syntax errors in one pass without creating
      semantically misleading AST nodes.
- [ ] Preserve deterministic ordering, source spans, severity, and recovery
      notes for every diagnostic.
- [ ] Add malformed-source corpus, fuzz, LSP, and CLI regression tests.

## Gate 22.2: Source architecture and documentation hygiene

- [ ] Decompose every production source file above the repository warning
      threshold by responsibility without creating generic utility modules.
- [ ] Keep all files below the hard architectural limit or record an approved
      exception with a decomposition plan.
- [ ] Define a meaningful public Rust API documentation policy for exported
      compiler, runtime, and tooling symbols.
- [ ] Enforce documentation coverage and broken-link checks in CI.

## Gate 22.3: Incremental and parallel compilation

- [ ] Define stable dependency, source, configuration, target, and toolchain
      cache identities.
- [ ] Invalidate only affected frontend, semantic, module, and codegen units.
- [ ] Prove clean-build equivalence between serial, incremental, and parallel
      modes.
- [ ] Bound cache storage, cancellation, stale artifacts, and failed builds.
- [ ] Add deterministic parallel scheduling and race regression tests.

## Gate 22.4: Debug information and developer workflow

- [ ] Define target-independent source identity and line mapping contracts.
- [ ] Emit DWARF or the target-native equivalent without exposing backend types
      to frontend modules.
- [ ] Prove breakpoints, stack traces, locals, ownership state, and generated
      source spans for native execution.
- [ ] Add a production REPL only after its ownership, module, cleanup, and
      diagnostic semantics are specified and tested.

## Gate 22.5: Target expansion

- [ ] Define target capability manifests for hosted, freestanding, and WASM
      profiles.
- [ ] Keep target-specific runtime services behind explicit profile contracts.
- [ ] Prove unsupported intrinsics and standard-library modules fail clearly
      before code generation.
- [ ] Add native object, execution, and no-host-runtime evidence per target.

## Gate 22.6: Language expansion decisions

- [ ] Specify closure capture ownership and cleanup before implementing
      closures or lambdas.
- [ ] Specify async task ownership, cancellation, scheduling, and target
      support before implementing async execution.
- [ ] Specify macro expansion phase boundaries, hygiene, diagnostics, and
      reproducibility before implementing macros.
- [ ] Each feature receives a separate ADR and cannot enter through a parser-
      only shortcut.

## Gate 22.7: Performance and acceptance

- [ ] Profile borrow checking and remove avoidable O(n*m) behavior while
      preserving diagnostics and deterministic ordering.
- [ ] Expand frontend, semantic, codegen, LSP, runtime, and packaging
      benchmarks with regression thresholds.
- [ ] Verify optimized builds are semantically equivalent to debug builds.
- [ ] Publish a release report containing benchmark deltas, target evidence,
      compatibility data, and known limitations.

## Completion criteria

Phase 22 closes only when every applicable gate has implementation evidence,
negative tests, documentation, and CI enforcement. Deferred language features
must remain explicitly deferred; no feature is considered production-ready
because a prototype parser accepts its syntax.
