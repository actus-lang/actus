# Phase 18: Strict Actus Conformance and Fail-Closed Compilation

Phase 18 makes the Actus compiler reject every unresolved correctness,
ownership, architecture, documentation, and execution violation. A successful
strict run is a conformance result, not merely evidence that a subset of the
source happened to compile.

The governing decision is
[`ADR-0041`](../decisions/ADR-0041-strict-actus-conformance.md). Phase 18
does not redesign ownership, change the compiler pipeline, or implement the
Phase 17 extension platform. It adds explicit validation and fail-closed
behavior around the existing language and runtime contracts.

## Phase status and sequencing

- [x] Adopt ADR-0041 as the governing design record for this phase.
- [ ] Inventory the current compiler, runtime, CLI, standard-library, and CI
      behavior before changing strictness.
- [ ] Complete the gates in numeric order unless a dependency is explicitly
      recorded in the gate's implementation notes.
- [ ] Keep Phase 17 implementation out of scope.
- [ ] Update `ROADMAP.md` only after this document is reviewed and present.
- [ ] Mark Phase 18 complete only after every final acceptance criterion is
      evidenced by a command, fixture, or CI artifact.

## Current implementation checkpoint

- [x] Add one non-repeatable `--strict` parser for `check`, `build`, and
      `test`.
- [x] Reject duplicate strict flags before configuration loading or source
      compilation.
- [x] Route strict configuration loading through a dedicated policy boundary.
- [x] Reject deprecated root and local dependency `Arca.toml` manifests in
      strict mode instead of emitting compatibility warnings.
- [x] Run source-aware semantic preflight before strict build code generation.
- [x] Run source-aware semantic validation while collecting strict meta tests.
- [x] Add positive and negative CLI/configuration regression tests for the
      implemented strict behaviors.
- [x] Introduce a renderer-independent diagnostic model with stable metadata
      and terminal rendering compatibility.
- [x] Define and use the initial strict configuration diagnostic codes
      `E1800` through `E1802`.
- [x] Sort diagnostics deterministically by source path, span, compiler phase,
      and stable code in the shared model and LSP adapter.
- [x] Add a stable JSON renderer that preserves shared diagnostic metadata
      without mutating the caller's ordering.
- [x] Add a colored terminal renderer whose disabled mode is identical to the
      plain renderer and whose colors are presentation-only.
- [x] Inventory the current frontend and semantic diagnostic catalog and
      reserve categorized strict ranges in `E1800`–`E1899`.
- [x] Reject duplicate diagnostic codes and contradictory severity metadata
      through the shared diagnostic catalog validator.
- [x] Route `actus check` and `actus build` frontend/semantic diagnostics
      through one source-aware CLI reporter.
- [x] Route strict `actus test` semantic preflight diagnostics through the
      shared diagnostic model with source-aware rendering.
- [x] Add negative execution tests for unknown strict forms and extra
      arguments across `check`, `build`, and `test`.
- [x] Make strict mode visible in successful `check`, `build`, and `test`
      command output.
- [x] Use the shared colored diagnostic renderer for interactive CLI errors
      while preserving plain output for non-terminal streams.

## Strict-mode contract

- [ ] `actus check --strict` validates source without producing executable
      artifacts.
- [ ] `actus build --strict` refuses code generation and linking after any
      error, warning promoted to error, or unresolved validation state.
- [ ] `actus test --strict` propagates compilation, test, timeout, signal, and
      non-zero test-process failures.
- [ ] Strict mode never inserts implicit conversions, ownership repairs,
      default targets, fallback declarations, or silent recovery behavior.
- [ ] Unsupported syntax, registered-but-unimplemented declarations, and
      unresolved metadata are hard failures.
- [ ] Normal interactive mode and strict conformance mode have an explicit,
      documented compatibility matrix.
- [ ] No source-level annotation can suppress a correctness, ownership,
      architecture, documentation, or execution failure.

## Gate 18.0: Baseline inventory and validation contract

### Compiler and runtime inventory

- [ ] Inventory every existing diagnostic code and identify duplicate or
      unstable meanings.
- [ ] Inventory warnings, parser recovery paths, semantic fallbacks, and
      code-generation placeholder paths.
- [ ] Inventory all CLI commands, exit statuses, filtering options, and
      target-selection paths.
- [ ] Inventory standard-library public declarations and current documentation
      coverage.
- [ ] Inventory source-limit checks, module-discovery checks, lockfile checks,
      and test-runner checks.
- [ ] Record every existing accepted fallback and classify it as supported,
      deprecated, or forbidden in strict mode.

### Conformance matrix

- [ ] Define the accepted behavior for check, build, and test in normal and
      strict modes.
- [ ] Define which diagnostics are errors in every command and mode.
- [ ] Define stable exit-status categories for source, configuration, test,
      runtime, and infrastructure failures.
- [ ] Define the minimum evidence required before a gate can be checked off.
- [ ] Add a review record for every intentional compatibility break.

## Gate 18.1: Deterministic diagnostics and strict error catalog

- [x] Reserve a documented diagnostic range for strict-conformance failures
      after checking the existing `E####` inventory.
- [x] Model severity, stable code, source path, half-open span, primary
      message, rule explanation, and actionable correction independently.
- [x] Define strict categories for frontend, semantic, architecture,
      documentation, source limits, and execution failures.
- [x] Reject duplicate diagnostic codes and contradictory severity metadata.
- [x] Sort diagnostics deterministically by source path, span, phase, and code.
- [ ] Ensure renderer choice cannot change validation, ordering, or exit code.
- [x] Add text, colored, JSON, and LSP diagnostic tests from the same fixture.
- [x] Add golden tests for stable messages and stable source locations.
- [x] Verify diagnostics remain deterministic across repeated clean runs.

## Gate 18.2: Strict CLI and configuration enforcement

- [x] Parse `--strict` consistently for `check`, `build`, and `test`.
- [x] Reject unknown, duplicated, conflicting, or misplaced strict-mode flags.
- [x] Ensure strict configuration is visible in command diagnostics and test
      metadata.
- [x] Prevent environment or workspace defaults from silently disabling strict
      validation.
- [x] Define the precedence of command-line, workspace, package, and target
      configuration.
- [x] Reject malformed configuration before source compilation begins.
- [x] Test successful and rejected invocations for every affected command.
- [x] Document the migration path from permissive checks to strict checks.

## Gate 18.3: Fail-closed compiler pipeline

- [x] Prevent parser recovery nodes from reaching semantic analysis as valid
      declarations or expressions.
- [x] Prevent semantic errors from reaching code generation or linking in the
      strict build flow.
- [ ] Reject empty declarations where the language contract requires
      implementation.
- [x] Reject incomplete role performances before code generation.
- [x] Reject duplicate top-level declarations before code generation.
- [x] Reject unreachable patterns during semantic validation.
- [ ] Reject placeholder declarations where the language contract requires
      implementation.
- [x] Reject unsupported constructs instead of emitting partial code.
- [ ] Ensure every accepted syntax form has a complete AST, semantic rule, and
      code-generation path, or an explicit non-codegen meaning.
- [ ] Remove implicit default branches that conceal unresolved compiler state.
- [x] Add a regression test proving a strict semantic failure emits no artifact.
- [x] Verify failed builds leave deterministic, bounded cleanup artifacts.

## Gate 18.4: Frontend and declaration completeness

- [ ] Reject unknown keywords, types, verbs, fields, modules, imports, and
      attributes with stable diagnostics.
- [ ] Reject malformed metadata and metadata attached to an invalid target.
- [ ] Reject duplicate declarations and ambiguous overload or sibling cases.
- [ ] Validate all return paths and reject missing-result branches.
- [ ] Validate generic parameters, bounds, instantiations, and unsupported
      combinations before code generation.
- [ ] Validate primitive widths, signedness, literal ranges, and overflow
      behavior without silent truncation.
- [ ] Add positive and negative fixtures for every declared frontend rule.
- [ ] Add a completeness report identifying validated and intentionally
      non-codegen declarations.

## Gate 18.5: Ownership, type, and exhaustiveness strictness

- [ ] Validate `erg` as the active mutable owner and reject unauthorized
      mutation through `abs`, `dat`, or `ins` bindings.
- [ ] Validate `abs` as a read-only, non-owning view with non-escaping lifetime
      propagation.
- [ ] Validate `dat` as terminal ownership transfer and reject post-transfer
      use, double transfer, or invalid restoration.
- [ ] Validate `ins` as an exclusive call-scope loan and reject aliasing,
      escaping, and invalid concurrent access.
- [ ] Reject use-after-move, use-after-drop, double-drop, invalid mutation, and
      cleanup-order violations.
- [ ] Enforce single-origin rules for borrowed views across returns, fields,
      iterators, arenas, and C-ABI boundaries.
- [ ] Require exhaustive `case` and `match` handling without silent defaults.
- [ ] Add negative tests for every ownership transition and every Result/Option
      branch that affects cleanup.
- [ ] Verify deterministic LIFO cleanup on success, error, early return, loop
      exit, and `?` propagation.

## Gate 18.6: Module facades and architectural boundaries

- [ ] Require every directory module to expose one canonical module-named
      `.act` facade.
- [ ] Reject external access to unexported siblings and imports that bypass the
      facade.
- [ ] Reject duplicate or ambiguous sibling declarations deterministically.
- [ ] Validate extensionless `open` exports and their visibility boundaries.
- [ ] Reject reverse compiler-pipeline dependencies.
- [ ] Reject parser ownership logic, AST backend types, semantic terminal
      rendering, and codegen semantic repair.
- [ ] Reject generic Actus files named `utils.act`, `helpers.act`, `common.act`,
      or `misc.act` unless an explicit architectural exception is recorded.
- [ ] Apply the same responsibility-based naming rule to Rust source modules,
      including `utils.rs`, `helpers.rs`, `common.rs`, and `misc.rs`.
- [ ] Add facade, sibling discovery, duplicate, visibility, and dependency
      direction fixtures.

## Gate 18.7: Public documentation conformance

- [ ] Discover every public Actus type, enum, field, performance, verb,
      external bridge, and facade export.
- [ ] Require a complete `"""` block docstring for each discovered public
      declaration where ADR-0025 applies.
- [ ] Require documentation of purpose, `erg`/`abs`/`dat`/`ins` ownership,
      lifetime or scope behavior, return variants, errors, mutation,
      allocation, cleanup, ABI, and platform effects when relevant.
- [ ] Reject missing, empty, one-line restatement, contradictory, or
      implementation-misrepresenting documentation.
- [ ] Require `///` documentation for public Rust APIs introduced by strict
      validation infrastructure.
- [ ] Add accepted and rejected documentation fixtures for each declaration
      category.
- [ ] Ensure documentation diagnostics point to the declaration and identify
      the missing contract section.

## Gate 18.8: Source structure and size conformance

- [ ] Treat files above 300 lines as requiring decomposition evidence.
- [ ] Treat files at or above 400 lines as strict failures requiring a split.
- [ ] Reject files above the 500-line hard limit.
- [ ] Treat functions above 30 lines as requiring decomposition evidence.
- [ ] Treat functions at or above 40 lines as strict failures requiring a
      split.
- [ ] Reject functions above the 60-line hard limit.
- [ ] Keep tests in `tests/` and Actus library fixtures in `tests/library/`;
      reject Rust logic embedded in Actus fixture trees.
- [ ] Define narrowly scoped exceptions only for generated or intentionally
      tabular artifacts, with owner, scope, reason, and replacement plan.
- [ ] Reject source-local comments or annotations that attempt to suppress
      architectural limits.

## Gate 18.9: Runtime, test runner, target, and lockfile integrity

- [ ] Require explicit C-ABI status contracts and prevent raw statuses from
      leaking through public Actus APIs.
- [ ] Validate target names and metadata before applying test or build filters.
- [ ] Reject malformed, conflicting, or incompatible target declarations.
- [ ] Make target filtering deterministic and verify referenced symbols exist.
- [ ] Require native runtime tests for every changed code-generation or ABI
      contract.
- [ ] Require positive and negative tests for every public standard-library
      contract affected by strict mode.
- [ ] Make test discovery, filtered counts, pass counts, failure counts, and
      exit codes exact and deterministic.
- [ ] Reject hangs, unbounded stdin waits, signal failures, and non-zero test
      processes as test failures.
- [ ] Validate lockfiles against manifests and source content before build or
      test execution.
- [ ] Verify deterministic artifacts and cleanup after both successful and
      failed runs.

## Gate 18.10: Standard-library migration and CI enforcement

- [ ] Run strict check over every `library/std` facade and sibling module.
- [ ] Add complete positive and negative strict fixtures under `tests/library/`.
- [ ] Verify every standard-library public declaration has the required block
      documentation and ownership contract.
- [ ] Verify standard-library tests use Actus fixtures rather than Rust logic
      embedded in the library test tree.
- [ ] Enable strict check, build, and test commands in local quality scripts.
- [ ] Enable the same strict commands in CI without duplicating validation
      rules in the CI configuration.
- [ ] Run the full matrix on Linux, macOS, and Windows.
- [ ] Verify colored, plain, JSON, and LSP diagnostics preserve the same
      result and ordering on every supported host.
- [ ] Record clean-checkout, reproducibility, artifact, coverage, and
      dependency/license evidence.

## Gate 18.11: Final conformance and release decision

- [ ] Re-run every accepted and rejected fixture from a clean checkout.
- [ ] Confirm no registered feature silently compiles without an implementation
      or explicit non-codegen contract.
- [ ] Confirm no semantic, ownership, module, documentation, limit, target,
      lockfile, or runner violation reaches a successful strict result.
- [ ] Confirm failed builds produce no usable executable artifact.
- [ ] Confirm diagnostics are deterministic across repeated and cross-platform
      runs.
- [ ] Confirm all required source and function limits are satisfied without
      undocumented exceptions.
- [ ] Update ADR-0041 with implementation evidence and any approved deviations.
- [ ] Update README, manifesto, roadmap, and CLI documentation so they agree
      with strict-mode behavior.
- [ ] Publish the final Phase 18 acceptance report and mark this phase complete.

## Required quality matrix

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets --all-features`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-targets --all-features`
- [ ] `scripts/check_source_limits.sh`
- [ ] `git diff --check`
- [ ] `actus check --strict` over the complete accepted fixture set.
- [ ] `actus build --strict` over native and supported cross-target fixtures.
- [ ] `actus test --strict` over positive, negative, timeout, and cleanup cases.
- [ ] Repeated-run comparison for diagnostics, exit codes, and artifact hashes.

## Completion invariant

Phase 18 is complete only when strict mode is fail-closed, deterministic, and
enforced for the standard library and CI; every accepted public contract has
positive and negative evidence; every rejected state has a stable diagnostic;
and a clean checkout cannot produce a successful result by relying on an
implicit fallback, incomplete implementation, undocumented public API,
architecture violation, or suppressed warning.
