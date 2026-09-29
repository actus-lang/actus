# Phase 19: Actus Alpha Core and Application Readiness

This phase turns the current compiler foundation into a reproducible Alpha
development environment for real Actus applications. It is governed by
[ADR-0045](../decisions/ADR-0045-alpha-core-and-application-readiness.md).

Phase 19 does not implement the Phase 17 extension platform. It completes the
language, standard-library, CLI, package, portability, and compatibility
contracts that the future platform is allowed to consume.

## Sequencing rules

- Work follows the gates in order: 19.0 through 19.6.
- Every new behavior requires accepted and rejected tests where applicable.
- Application fixtures belong in `examples/` and `tests/`; implementation
  logic must not be hidden inside a fixture.
- A gate is not complete because a source file parses. Native execution,
  diagnostics, cleanup, and artifact behavior must be evidenced at the
  relevant boundary.
- Phase 17 remains blocked until the Alpha handoff gate is complete or its
  dependency is explicitly re-approved.

## Gate 19.0: Baseline and Contract Inventory

### Inventory

- [ ] List every unchecked Phase 16 Gate 6 item and assign it to a later gate.
- [ ] List every unchecked Phase 11 struct item and assign it to a later gate.
- [ ] Inventory all public compiler, CLI, runtime, standard-library, and
      package contracts currently used by examples and tests.
- [ ] Identify duplicated, contradictory, or stale claims in README,
      MANIFESTO, language references, ADRs, and roadmaps.
- [ ] Record current supported host targets, linkers, and known platform
      substitutions.

### Alpha baseline

- [ ] Define the accepted Alpha language edition and compiler invocation.
- [ ] Freeze the currently accepted ownership roles and cleanup guarantees.
- [ ] Freeze the accepted ADR-0042, ADR-0043, and ADR-0044 operator surface.
- [ ] Freeze the supported `std::io`, `std::fs`, and `std::path` public surface.
- [ ] Define which existing features remain experimental or deferred.
- [ ] Add a baseline compatibility report with compiler and repository hashes.

### Gate 19.0 evidence

- [ ] Every remaining item is `implemented`, `scheduled`, or `deferred`.
- [ ] The baseline report is reproducible from a clean checkout.
- [ ] Documentation contradictions are either corrected or recorded as work.

## Gate 19.1: Complete Struct Semantics

### Field access and ownership

- [ ] Define module and field visibility rules.
- [ ] Define `erg` field ownership and mutation rules.
- [ ] Define `abs` field access and frozen-view propagation.
- [ ] Define `dat` field moves and the containing-owner state afterward.
- [ ] Define whether `ins` may loan a field or indexed field slot.
- [ ] Reject mutation through frozen or moved field paths.

### Copy, move, and assignment

- [ ] Define whole-struct copy eligibility.
- [ ] Define whole-struct move behavior and use-after-move diagnostics.
- [ ] Define field assignment compatibility and replacement cleanup.
- [ ] Define partial moves and the remaining cleanup plan.
- [ ] Reject overlapping moves and ambiguous ownership joins.
- [ ] Add positive and negative semantic fixtures for each transition.

### References and representation

- [ ] Define which reference-bearing fields are permitted in Alpha.
- [ ] Reject self-referential and escaping borrow fields without a lifetime
      contract.
- [ ] Define packed and externally represented struct policies.
- [ ] Define native layout, alignment, and padding guarantees.
- [ ] Define public struct C-ABI exposure or explicitly defer it.
- [ ] Add layout and cleanup golden tests for accepted representations.

### Gate 19.1 evidence

- [ ] Struct visibility and ownership behavior has stable diagnostics.
- [ ] Accepted and rejected struct programs pass semantic and native tests.
- [ ] Struct documentation and the language reference match implementation.

## Gate 19.2: Standard Library and Real Applications

### Console application

- [ ] Create a real console application using `std::io` readers and writers.
- [ ] Exercise successful input, output, flush, and EOF handling.
- [ ] Exercise typed I/O failure propagation and cleanup.
- [ ] Verify stdout, stderr, and process exit status independently.
- [ ] Document the complete source-to-running-program workflow.

### File utility

- [ ] Create a real file utility using `std::fs` and `std::path`.
- [ ] Exercise path construction, inspection, normalization, and platform
      boundary behavior.
- [ ] Exercise file creation, reading, writing, metadata, and removal.
- [ ] Exercise missing-file, invalid-path, and permission failure mappings.
- [ ] Verify all handles, buffers, paths, and temporary resources are cleaned
      deterministically on success and failure.

### Application evidence

- [ ] Keep application source in `examples/` and executable checks in `tests/`.
- [ ] Add positive and negative application fixtures.
- [ ] Compile both applications through the native backend.
- [ ] Run both applications and record output and exit-code evidence.
- [ ] Reject applications that pass only parser or semantic validation.

### Gate 19.2 evidence

- [ ] Two real applications build and run from a clean checkout.
- [ ] Their documented failure paths return typed errors and expected status.
- [ ] No raw host/C status leaks through the public standard-library API.

## Gate 19.3: CLI, Package, and Reproducible Workflow

### CLI commands

- [ ] Verify `actus check` validates without emitting native code.
- [ ] Verify `actus build` emits the documented artifact types and names.
- [ ] Verify `actus run` builds, executes, forwards output, and reports status.
- [ ] Complete deterministic `actus test` discovery and reporting.
- [ ] Complete `actus fmt` and `actus fmt --check` behavior.
- [ ] Verify `actus watch --once` and continuous watch behavior.
- [ ] Reject unknown, repeated, misplaced, and conflicting command options.

### Packages and modules

- [ ] Verify manifest entry resolution from project root and nested paths.
- [ ] Verify facade imports and sibling declarations from clean workspaces.
- [ ] Verify local dependency resolution and lockfile content checksums.
- [ ] Reject missing, stale, conflicting, and unsafe dependency metadata.
- [ ] Define deterministic package and module failure diagnostics.

### Profiles and artifacts

- [ ] Verify debug and release profile selection.
- [ ] Verify target profile and linker configuration behavior.
- [ ] Define artifact directory, naming, and failed-build cleanup contracts.
- [ ] Build identical inputs twice and compare artifacts where determinism is
      promised by the target contract.
- [ ] Verify no failed command leaves a misleading executable or object file.

### Gate 19.3 evidence

- [ ] CLI workflow tests cover accepted and rejected command forms.
- [ ] Package and lockfile tests pass from isolated temporary workspaces.
- [ ] Repeated builds produce the documented deterministic result.

## Gate 19.4: Runtime, Portability, and Failure Matrix

### Runtime boundaries

- [ ] Verify every public fallible I/O operation returns a typed Actus result.
- [ ] Verify raw POSIX, Windows, and linker statuses remain private to runtime
      bridges.
- [ ] Verify C-ABI status contracts use documented explicit values.
- [ ] Verify no hidden allocation is introduced in zero-allocation contracts.
- [ ] Verify ownership restoration across runtime failures and early returns.

### Cleanup and control flow

- [ ] Verify LIFO cleanup for normal return, early return, and `?`.
- [ ] Verify cleanup across `break`, `continue`, and nested `case` branches.
- [ ] Verify `erg`, `abs`, `dat`, and `ins` state transitions after failed calls.
- [ ] Verify path, buffer, file, and stream resources are not double-dropped.
- [ ] Add regression tests for every discovered cleanup or ABI failure.

### Supported hosts

- [ ] Run the compiler and native application matrix on Linux.
- [ ] Run the compiler and native application matrix on macOS.
- [ ] Run the compiler and native application matrix on Windows.
- [ ] Verify host-native path separators, encoding boundaries, line endings,
      executable names, and linker selection.
- [ ] Record bare-metal substitutions and unavailable host services explicitly.

### Gate 19.4 evidence

- [ ] Platform-specific claims have command output or native execution evidence.
- [ ] Failure behavior is deterministic and diagnostically actionable.
- [ ] No host-specific workaround is hidden in shared compiler layers.

## Gate 19.5: Alpha Compatibility and Documentation

### Compatibility policy

- [ ] Define the Alpha edition and supported compiler command line.
- [ ] Classify changes as compatible, diagnostic-only, edition-gated, or
      breaking.
- [ ] Define diagnostic-code stability and rendering compatibility.
- [ ] Define standard-library API compatibility and deprecation rules.
- [ ] Define lockfile and artifact compatibility expectations.
- [ ] Define the policy for experimental and deferred features.

### Documentation

- [ ] Document ownership roles with call-site examples.
- [ ] Document standard-library error and cleanup contracts.
- [ ] Document supported targets and target-specific substitutions.
- [ ] Document package, module facade, build, run, test, and format workflows.
- [ ] Add beginner-to-running-program tutorials based on verified commands.
- [ ] Add accepted and rejected examples for the Alpha language surface.
- [ ] Synchronize README, MANIFESTO, language references, ADRs, and roadmap.

### Gate 19.5 evidence

- [ ] Public documentation describes only implemented behavior.
- [ ] Every documented command is reproduced in an automated or recorded test.
- [ ] Compatibility and diagnostic policies are reviewable without source-code
      archaeology.

## Gate 19.6: Alpha Release Candidate and Handoff

### Clean-checkout acceptance

- [ ] Re-run all Rust quality checks from a clean checkout.
- [ ] Re-run all Actus strict, semantic, native, and application tests.
- [ ] Re-run standard-library, package, and lockfile checks.
- [ ] Re-run Linux, macOS, and Windows CI checks.
- [ ] Re-run coverage and dependency/license policy checks.
- [ ] Verify source and function size limits and `git diff --check`.

### Reproducibility

- [ ] Recreate lockfiles from clean workspaces and compare canonical output.
- [ ] Rebuild release artifacts twice under the same target contract.
- [ ] Record compiler version, target, profile, source revision, and artifact
      checksums in the acceptance report.
- [ ] Ensure the acceptance report contains real command output or linked CI
      evidence rather than unverified claims.

### Handoff

- [ ] Record every remaining deferral and its intended future phase.
- [ ] Freeze the compiler/LSP contracts that Phase 17 may consume.
- [ ] Reject extension-side semantic reimplementation in the handoff record.
- [ ] Mark Phase 19 complete only after every Alpha criterion is satisfied.
- [ ] Update `ROADMAP.md` and all affected phase/ADR references.

## Phase 19 invariant

- [ ] A real Actus application can be checked, built, run, tested, formatted,
      and reproduced through documented commands.
- [ ] Struct ownership, layout, cleanup, and ABI rules are explicit.
- [ ] Public standard-library failures are typed and platform-independent.
- [ ] No implicit conversion, hidden allocation, or undocumented ownership
      transition is required by accepted Alpha programs.
- [ ] Phase 17 can consume versioned compiler contracts without becoming a
      second compiler.

## Completion criteria

- [ ] Gates 19.0 through 19.6 are complete.
- [ ] Phase 16 Gate 6 is complete or all deferrals are explicitly accepted.
- [ ] Remaining Phase 11 struct items are complete or explicitly deferred.
- [ ] At least two real Actus applications pass native execution checks.
- [ ] Supported host quality evidence is recorded.
- [ ] Alpha compatibility and diagnostic policies are published.
- [ ] A clean checkout reproduces the documented acceptance report.
