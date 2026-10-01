# Phase 21: Production Language Capability and Wire Readiness

This phase proves that Actus is strong enough to express production-grade,
bounded, deterministic binary algorithms before `std::wire` implementation
begins. It validates the language and compiler as one system; it does not
implement Wire or Ustari.

The phase follows [ADR-0053](../decisions/ADR-0053-production-language-capability-and-wire-readiness.md).
All gates begin open. A checkbox may be marked only after the listed evidence
exists in the repository and the relevant checks pass.

## Gate 21.0: Baseline and evidence contract

- [ ] Record the exact compiler revision, target profiles, runtime profiles,
      and quality commands used for Phase 21.
- [ ] Inventory existing evidence for constants, control flow, operators,
      arrays, buffers, packs, ownership, results, modules, and LSP.
- [ ] Identify every known limitation that would block a production-shaped
      bounded codec fixture.
- [ ] Add a deterministic evidence index mapping each gate to test files and
      commands.
- [ ] Confirm no temporary C bridge, handwritten compiler exception, or
      application-specific lowering is used by the phase fixtures.
- [ ] Classify every production `unwrap`, `expect`, `unreachable`, and
      equivalent panic site as test-only, private-invariant-only, or
      production-risk.
- [ ] Convert audit findings into owned work items with evidence commands and
      closure conditions; audit reports are inputs, not implementation proof.

## Gate 21.1: Structured control flow and cleanup

- [ ] Prove native `if/else` and expression conditionals with nested ownership
      joins and diverging branches.
- [ ] Prove `case`-nested `return`, `break`, and `continue` with loop-carried
      values.
- [ ] Prove nested loops and case branches preserve deterministic cleanup on
      normal exit, `break`, `continue`, `return`, and failure paths.
- [ ] Add repeated native builds whose output and exit behavior are identical.
- [ ] Add negative tests for control-flow escape outside a loop and invalid
      ownership joins.

## Gate 21.2: Fixed-width arithmetic and checked numeric behavior

- [ ] Prove direct same-type arithmetic for supported signed and unsigned
      widths, including compound assignment.
- [ ] Prove relational, equality, remainder, bitwise, and shift semantics at
      declared widths and signedness.
- [ ] Prove typed and type-directed literals preserve source spans and reject
      overflow or underflow.
- [ ] Prove explicit casts reject incompatible types and trap runtime range
      failures deterministically.
- [ ] Prove native lowering does not silently widen, sign-flip, or promote
      values across an operator boundary.

## Gate 21.3: Bounded memory and zero-copy ownership

- [ ] Prove `Array[T, N]` contiguous layout, dynamic unsigned indexing, and
      compile-time and runtime bounds failures.
- [ ] Prove `Buffer` indexed reads and writes with first, last, empty, and
      out-of-bounds cases.
- [ ] Prove `pack` field reads and in-place writes through native lowering,
      including range checks for narrow fields.
- [ ] Prove `ins` array and buffer mutation changes the caller-owned storage
      and restores the caller to an active mutable state.
- [ ] Prove `abs` views do not allocate or consume the source and `dat`
      transfers have exactly one cleanup owner.
- [ ] Add freestanding object evidence showing no hosted allocation or runtime
      service is introduced by these operations.

## Gate 21.4: Typed failure and module boundaries

- [ ] Prove public operations expose typed `Result[T, E]` contracts with
      documented success counts and failure variants.
- [ ] Prove `?` propagation preserves cleanup and ownership on every early
      failure path.
- [ ] Prove raw C ABI statuses cannot cross a public facade untranslated.
- [ ] Prove facade exports distinguish public, private, missing, duplicate,
      and malformed declarations with stable diagnostics.
- [ ] Prove repeated failures produce deterministic diagnostic ordering and
      source spans.

## Gate 21.5: Compile-time data and reusable abstractions

- [ ] Prove named constants support protocol-style masks, limits, widths, and
      layout values without runtime storage.
- [ ] Reject constant cycles, runtime dependencies, invalid widths, and
      out-of-range constant expressions with stable diagnostics.
- [ ] Prove generic or role-based reusable helpers can express a bounded
      algorithm without duplicating one function per scalar width.
- [ ] Prove specialization and native symbol generation remain deterministic.
- [ ] Verify public declarations retain complete documentation contracts.

## Gate 21.6: Tooling and compiler parity

- [ ] Add parser acceptance and rejection fixtures for every Phase 21 syntax
      exercised by the capability suite.
- [ ] Add semantic ownership, type, bounds, and failure diagnostics for every
      negative fixture.
- [ ] Add native codegen execution and object-level tests for every runtime
      claim.
- [ ] Add formatter round-trip and idempotence tests without moving or
      deleting Actus docstrings or module declarations.
- [ ] Add LSP diagnostics, semantic model, hover, completion, and navigation
      evidence for the exercised public surface.
- [ ] Replace production panic paths reachable from source, package, module,
      runtime, or JSON-RPC input with typed errors or validated private
      invariants.
- [ ] Add LSP liveness tests for missing documents, missing request ids,
      malformed requests, stale versions, invalid ranges, and unsupported
      methods.
- [ ] Add compiler liveness tests for malformed AST recovery, invalid module
      state, malformed manifests/lockfiles, and unavailable target contracts.
- [ ] Add runtime negative tests for null, stale, malformed, and out-of-bounds
      handles without process termination.
- [ ] Pass `cargo fmt --all -- --check`, `cargo check --all-targets
      --all-features`, `cargo clippy --all-targets --all-features -- -D
      warnings`, `cargo test --all-targets --all-features`,
      `scripts/check_source_limits.sh`, and `git diff --check`.

## Gate 21.7: Documentation and product truthfulness

- [ ] Correct README and root roadmap phase status so completed, active,
      deferred, and planned work are distinguishable.
- [ ] Audit MANIFESTO and language guides against implemented syntax and
      runtime behavior; move aspirational features to an explicit future
      vision section.
- [ ] Expand compiler architecture documentation to cover AST, semantic
      ownership/borrowing, type resolution, cleanup planning, Cranelift
      lowering, module/build graph, diagnostics, formatter, and LSP flow.
- [ ] Add standard-library documentation for every public module and
      declaration, including ownership roles, `Result` errors, side effects,
      allocation behavior, and C ABI boundaries.
- [ ] Add an executable examples index with invocation, runtime profile,
      expected output, and feature coverage for every supported example.
- [ ] Add documentation consistency checks for broken links, duplicate ADR
      identifiers, stale roadmap references, unsupported feature claims, and
      undocumented public APIs.
- [ ] Verify public documentation describes implemented behavior and does not
      use an audit report or aspirational text as implementation evidence.

## Gate 21.8: Production-shaped codec readiness

- [ ] Write a small bounded codec fixture using only public Actus language
      and standard-library capabilities; do not call it Wire yet.
- [ ] Require fixed-width header encoding, payload length validation, checksum
      calculation, malformed input rejection, and typed failure propagation.
- [ ] Prove zero-copy caller-owned buffers and deterministic cleanup on success
      and every rejected input path.
- [ ] Prove repeated native builds are byte-for-byte deterministic where the
      artifact contract permits comparison.
- [ ] Prove hosted execution, freestanding object generation, and diagnostics
      agree on the same fixture.
- [ ] Record a final readiness report naming all evidence and explicitly
      authorize or reject starting `std::wire` Gate 1.

## Gate 21.9: Enterprise release acceptance

- [ ] Verify clean reproducible builds for hosted and supported freestanding
      target profiles from a fresh checkout.
- [ ] Verify compiler, formatter, LSP, runtime, and standard-library version
      metadata comes from one authoritative source.
- [ ] Verify release artifacts contain the compiler, runtime profiles,
      standard-library references, examples index, licenses, and target
      declarations.
- [ ] Verify no audit scratch file, local path, secret, temporary bridge, or
      untracked implementation file enters the release package.
- [ ] Run the complete CI-equivalent quality suite on every release target and
      preserve machine-readable evidence for the release record.
- [ ] Obtain explicit architecture review sign-off before opening Wire Gate 1.

## Enterprise platform work outside the Wire prerequisite

The audit also identifies capabilities that are important for a world-class
compiler but are not required to express the bounded Wire-readiness fixture.
They are not silently discarded or marked complete here; they are tracked with
independent gates in [Phase 22](phase-22-enterprise-compiler-platform.md):

- parser multi-error recovery;
- file decomposition and source-level documentation coverage;
- incremental and parallel compilation;
- DWARF/debugger support and additional targets such as WASM;
- REPL and developer workflow expansion;
- borrow-checking complexity and benchmark expansion;
- closures, lambdas, async, and macros after separate language decisions.

## Deferred work policy

Anything unavailable on a local host must remain unchecked, not be marked
complete. A deferred item must state its owner, reason, required environment,
and exact command or artifact that will close it. Deferral never authorizes a
temporary implementation or a weakened acceptance criterion.
