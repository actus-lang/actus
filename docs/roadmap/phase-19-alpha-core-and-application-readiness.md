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

- [x] List every unchecked Phase 16 Gate 6 item and assign it to a later gate.
- [x] List every unchecked Phase 11 struct item and assign it to a later gate.
- [x] Inventory all public compiler, CLI, runtime, standard-library, and
      package contracts currently used by examples and tests.
- [x] Identify duplicated, contradictory, or stale claims in README,
      MANIFESTO, language references, ADRs, and roadmaps.
- [x] Record current supported host targets, linkers, and known platform
      substitutions.

### Alpha baseline

- [x] Define the accepted Alpha language edition and compiler invocation.
- [x] Freeze the currently accepted ownership roles and cleanup guarantees.
- [x] Freeze the accepted ADR-0042, ADR-0043, and ADR-0044 operator surface.
- [x] Freeze the supported `std::io`, `std::fs`, and `std::path` public surface.
- [x] Define which existing features remain experimental or deferred.
- [x] Add a baseline compatibility report with compiler and repository hashes.

### Gate 19.0 evidence

- [x] Every remaining item is `implemented`, `scheduled`, or `deferred`.
- [x] The baseline report is reproducible from a clean checkout.
- [x] Documentation contradictions are either corrected or recorded as work.

## Gate 19.1: Complete Struct Semantics

### Field access and ownership

- [x] Define module and field visibility rules: `open` declarations cross
      facade boundaries; fields have no independent visibility modifier.
- [x] Define `erg` field ownership and mutation rules.
- [x] Define `abs` field access and frozen-view propagation.
- [x] Define `dat` field moves and the containing-owner state afterward.
- [x] Define whether `ins` may loan a field or indexed field slot: persistent
      `ins` fields are rejected; loans remain call-scope operations.
- [x] Reject mutation through frozen or moved field paths.

### Copy, move, and assignment

- [x] Define whole-struct copy eligibility: Alpha structs are not implicitly
      copyable; explicit aggregate assignment transfers ownership.
- [x] Define whole-struct move behavior and use-after-move diagnostics.
- [x] Define field assignment compatibility and replacement cleanup.
- [x] Define partial moves and the remaining cleanup plan.
- [x] Reject overlapping moves and ambiguous ownership joins.
- [x] Add positive and negative semantic fixtures for each transition.

### References and representation

- [x] Define which reference-bearing fields are permitted in Alpha.
- [x] Reject self-referential and escaping borrow fields without a lifetime
      contract.
- [x] Define packed and externally represented struct policies.
- [x] Define native layout, alignment, and padding guarantees.
- [x] Define public struct C-ABI exposure or explicitly defer unsupported
      aggregate layouts.
- [x] Add layout and cleanup golden tests for accepted representations.

### Gate 19.1 evidence

- [x] Struct visibility and ownership behavior has stable diagnostics.
- [x] Accepted and rejected struct programs pass semantic and native tests.
- [x] Struct documentation and the language reference match implementation.

## Gate 19.2: Standard Library and Real Applications

### Console application

- [x] Create a real console application using `std::io` readers and writers.
- [x] Exercise successful input, output, flush, and EOF handling.
- [x] Exercise typed I/O failure propagation and cleanup.
- [x] Verify stdout, stderr, and process exit status independently.
- [x] Document the complete source-to-running-program workflow.

### File utility

- [x] Create a real file utility using `std::fs` and `std::path`.
- [x] Exercise path construction, inspection, normalization, and platform
      boundary behavior.
- [x] Exercise file creation, reading, writing, metadata, and removal.
- [x] Exercise missing-file and invalid-path failure mappings through typed
      `IoError` and `PathError` results; defer the host permission matrix to
      Gate 19.4.
- [x] Verify all handles, buffers, paths, and temporary resources are cleaned
      deterministically on success and failure.

### Application evidence

- [x] Keep application source in `examples/` and executable checks in `tests/`.
- [x] Add positive and negative application fixtures.
- [x] Compile both applications through the native backend.
- [x] Run both applications and record output and exit-code evidence.
- [x] Reject applications that pass only parser or semantic validation.

### Gate 19.2 evidence

- [x] Two real applications build and run from a clean checkout.
- [x] Their documented failure paths return typed errors and expected status.
- [x] No raw host/C status leaks through the public standard-library API.

## Gate 19.3: CLI, Package, and Reproducible Workflow

### CLI commands

- [x] Verify `actus check` validates without emitting native code.
- [x] Verify `actus build` emits the documented artifact types and names.
- [x] Verify `actus run` builds, executes, forwards output, and reports status.
- [x] Complete deterministic `actus test` discovery and reporting.
- [x] Complete `actus fmt` and `actus fmt --check` behavior.
- [x] Verify `actus watch --once` and continuous watch behavior.
- [x] Reject unknown, repeated, misplaced, and conflicting command options.

### Packages and modules

- [x] Verify manifest entry resolution from project root and nested paths.
- [x] Verify facade imports and sibling declarations from clean workspaces.
- [x] Verify local dependency resolution and lockfile content checksums.
- [x] Reject missing, stale, conflicting, and unsafe dependency metadata.
- [x] Define deterministic package and module failure diagnostics.

### Profiles and artifacts

- [x] Verify debug and release profile selection.
- [x] Verify target profile and linker configuration behavior.
- [x] Define artifact directory, naming, and failed-build cleanup contracts.
- [x] Build identical inputs twice and compare artifacts where determinism is
      promised by the target contract.
- [x] Verify no failed command leaves a misleading executable or object file.

### Gate 19.3 evidence

- [x] CLI workflow tests cover accepted and rejected command forms.
- [x] Package and lockfile tests pass from isolated temporary workspaces.
- [x] Repeated builds produce the documented deterministic result.
- [x] Missing-facade diagnostics are stable across repeated isolated checks.

## Gate 19.4: Runtime, Portability, and Failure Matrix

### Runtime boundaries

- [ ] Verify every public fallible I/O operation returns a typed Actus result.
- [ ] Verify raw POSIX, Windows, and linker statuses remain private to runtime
      bridges.
- [x] Verify C-ABI status contracts use documented explicit values.
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

The initial runtime boundary slice is covered by the explicit null-handle
failure matrix in `tests/runtime_failure_matrix.rs`. The remaining runtime,
cleanup, and host-matrix items require separate evidence and remain open.

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
