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

### Runtime profiles and standard-library resolution

The project manifest must select the runtime environment explicitly. The
runtime profile is a build configuration, not an ordinary user dependency:

```toml
[build]
runtime = "std"
```

- [ ] Define the manifest schema for `runtime = "core" | "std" |
      "freestanding"`, including the default profile and edition-aware
      compatibility rules.
- [ ] Keep `core` implicit and dependency-free; it must not require users to
      spell out a path to compiler-owned runtime sources.
- [ ] Make `std` opt in through the manifest and resolve the compiler-owned
      standard-library package from a versioned builtin/sysroot location.
- [ ] Define canonical standard-library imports and facades so a project can
      use `std::io`, `std::fs`, and `std::path` without copying the library
      into its own `src/` tree or configuring absolute host paths.
- [ ] Preserve explicit local dependency resolution for third-party and
      workspace packages, while rejecting ambiguous aliases or declarations
      that conflict with the selected runtime profile.
- [ ] Build only standard-library modules reachable from the project's import
      graph; unused `std` modules must not be compiled or linked implicitly.
- [ ] Include runtime profile, standard-library version, target, and profile
      in lockfile validation and artifact identity so builds remain
      reproducible across machines.
- [ ] Define diagnostics for missing runtime metadata, unsupported runtime
      values, incompatible imports, and attempts to use `std` from a
      freestanding build.
- [ ] Add `actus init` templates that declare the intended runtime profile and
      produce a valid first build without manual standard-library wiring.
- [ ] Add accepted and rejected manifest tests for `core`, `std`, and
      `freestanding`, including lockfile mismatch and conflicting dependency
      cases.
- [ ] Add native integration tests proving that `std` applications compile
      and run, while `core` and `freestanding` builds do not acquire a host
      runtime implicitly.
- [ ] Document the runtime selection and standard-library resolution contract
      in the user-facing project and package documentation.

The implementation should be split into a future architecture decision for
runtime selection and builtin package resolution, followed by configuration
loading, module resolution, reachability-based compilation, lockfile identity,
CLI templates, and end-to-end verification. This gate must not be marked
complete while a project still depends on manually copied `library/std`
paths.

## Gate 19.4: Runtime, Portability, and Failure Matrix

### Runtime boundaries

- [x] Verify every public fallible I/O operation returns a typed Actus result.
- [deferred] Verify raw POSIX, Windows, and linker statuses remain private to
      runtime bridges. Deferred to Phase 20 Gate 20.3 and Gate 20.4, where
      module-internal scope and facade-bounded visibility will be implemented
      under ADR-0047.
- [x] Verify C-ABI status contracts use documented explicit values.
- [x] Verify no hidden allocation is introduced in zero-allocation contracts.
- [x] Verify ownership restoration across runtime failures and early returns.

### Cleanup and control flow

- [x] Verify LIFO cleanup for normal return, early return, and `?`.
- [x] Verify cleanup across `break`, `continue`, and nested `case` branches.
- [x] Verify `erg`, `abs`, `dat`, and `ins` state transitions after failed calls.
- [x] Verify path, buffer, file, and stream resources are not double-dropped.
- [x] Add regression tests for every discovered cleanup or ABI failure.

### Supported hosts

- [x] Run the compiler and native application matrix on Linux.
- [x] Run the compiler and native application matrix on macOS.
- [x] Run the compiler and native application matrix on Windows.
- [x] Verify host-native path separators, encoding boundaries, line endings,
      executable names, and linker selection.
- [deferred] Record bare-metal substitutions and unavailable host services
      explicitly. Deferred to a future embedded/bare-metal phase; Phase 19
      acceptance is limited to hosted Linux, macOS, and Windows targets.

### Gate 19.4 evidence

- [x] Platform-specific claims have command output or native execution evidence
      for the verified Linux host slice.
- [x] Failure behavior is deterministic and diagnostically actionable.
- [x] No host-specific workaround is hidden in shared compiler layers.

The initial runtime boundary slice is covered by the explicit null-handle
failure matrix in `tests/runtime_failure_matrix.rs`. The Linux host slice was
verified with `./scripts/check_stdlib.sh` and
`cargo test --all-targets --all-features -- --test-threads=1`; both completed
successfully, including native application, runtime, filesystem, path, and
standard-library suites. GitHub Actions then passed the corresponding macOS
and Windows native matrices on PR #40, including the file utility's
host-native path construction.

Borrowed path inspection is covered by `tests/runtime_zero_alloc.rs`, which
counts allocations around predicates, component views, iteration, and C-view
construction. Builders are intentionally excluded because their explicit
capacity-growth contract permits reallocation.

The public fallible standard-library surface is audited by
`tests/stdlib_result_surface.rs`: every open fallible verb and public
performance method returns `Result` or `Option`, while explicitly infallible
constructors, predicates, configuration helpers, and drop operations remain
value- or unit-returning by contract.

Raw platform and linker status privacy is explicitly deferred to Phase 20.
The current flattened module-import representation cannot both keep private
runtime bridges available to standard-library wrappers and hide them from
external Actus callers. ADR-0047 defines the required module compilation-unit
and public-interface split; Gate 19.4 must not claim this invariant until that
work is implemented and tested.

Target selection is covered by
`tests/target.rs::target_contracts_select_platform_behavior_without_host_cfg_branches`.
The test verifies Linux, macOS, Windows MSVC, Windows GNU, and freestanding
target contracts through `TargetSpec`, including linker flavor, entry contract,
and platform selectors. Shared compiler target behavior is therefore derived
from the target contract rather than from the host operating system.

Native ownership restoration after a typed failure is covered by
`tests/buffer_cli.rs::restores_an_ins_buffer_after_a_typed_failure_return`.
The callee mutates an `ins Buffer`, returns `Err`, and the caller successfully
reuses the restored owner afterward.

Cleanup evidence is distributed across `tests/semantic/`, `tests/codegen/`,
`tests/buffer_cli.rs`, `tests/fs.rs`, `tests/std_io/`, and `tests/cli.rs`;
these tests cover ownership restoration, LIFO unwinding, nested control flow,
and native resource cleanup.

## Gate 19.5: Alpha Compatibility and Documentation

### Compatibility policy

- [x] Define the Alpha edition and supported compiler command line.
- [x] Classify changes as compatible, diagnostic-only, edition-gated, or
      breaking.
- [x] Define diagnostic-code stability and rendering compatibility.
- [x] Define standard-library API compatibility and deprecation rules.
- [x] Define lockfile and artifact compatibility expectations.
- [x] Define the policy for experimental and deferred features.

### Documentation

- [x] Document ownership roles with call-site examples.
- [x] Document standard-library error and cleanup contracts.
- [x] Document supported targets and target-specific substitutions.
- [x] Document package, module facade, build, run, test, and format workflows.
- [x] Add beginner-to-running-program tutorials based on verified commands.
- [x] Add accepted and rejected examples for the Alpha language surface.
- [x] Synchronize README, MANIFESTO, language references, ADRs, and roadmap.

### Gate 19.5 evidence

- [x] Public documentation describes only implemented behavior.
- [x] Every documented command is reproduced in an automated or recorded test.
- [x] Compatibility and diagnostic policies are reviewable without source-code
      archaeology.

## Gate 19.6: Alpha Release Candidate and Handoff

### Clean-checkout acceptance

- [x] Re-run all Rust quality checks from a clean checkout.
- [x] Re-run all Actus strict, semantic, native, and application tests.
- [x] Re-run standard-library, package, and lockfile checks.
- [x] Re-run Linux, macOS, and Windows CI checks.
- [x] Re-run coverage checks.
- [x] Re-run dependency and license policy checks.
- [x] Verify source and function size limits and `git diff --check`.

### Reproducibility

The local Gate 19.6 acceptance run was executed from a clean working tree
after commit `b002d55`. `cargo test --all-targets --all-features` passed all
Rust unit and integration suites, including native applications. The strict
Actus runner passed 37 tests, standard-library conformance and documentation
tests passed, `actus lock --check` passed, and source-limit and diff checks
passed. This is working-tree evidence, not yet a fresh-checkout or
cross-platform release-candidate result.

A detached clean worktree at commit `d8885a0` independently passed formatting,
compilation, all Rust tests, source limits, diff validation, standard-library
conformance, strict Actus tests, and lockfile validation. Coverage was run
with `cargo-llvm-cov 0.9.1` and produced an LCOV report with 41,209 lines.
The local acceptance environment was Rust `1.98.0` for
`x86_64-unknown-linux-gnu`; the release object checksum recorded below was
identical across both builds.

`cargo deny check advisories licenses bans sources` passed using an isolated
Cargo home; advisory, ban, license, and source policies are green. Two
`--release --emit obj` builds of `examples/hello.act` produced byte-identical
objects with SHA-256
`a1c361bcaf8abc697517675f4036f2669a51fc7fb5ce4a30611c4bba0d63cf5f`.

In a separate detached clean worktree, `actus lock` regenerated `Actus.lock`,
`actus lock --check` accepted it, and `git diff --exit-code -- Actus.lock`
confirmed canonical lockfile stability.

- [x] Recreate lockfiles from clean workspaces and compare canonical output.
- [x] Rebuild release artifacts twice under the same target contract.
- [x] Record compiler version, target, profile, source revision, and artifact
      checksums in the acceptance report.
- [x] Ensure the acceptance report contains real command output or linked CI
      evidence rather than unverified claims.

PR #40 records the remote acceptance evidence: Linux, macOS, and Windows
Rust checks, coverage, and dependency/license policy all passed. The Windows
application matrix initially exposed a POSIX path construction defect; the
follow-up fix switched the example to `path_from_ascii`, after which the
application suite passed on all supported hosted targets.

### Handoff

- [x] Record every remaining deferral and its intended future phase.
- [x] Freeze the compiler/LSP contracts that Phase 17 may consume.
- [x] Reject extension-side semantic reimplementation in the handoff record.
- [x] Mark Phase 19 complete only after every Alpha criterion is satisfied.
- [x] Update `ROADMAP.md` and all affected phase/ADR references.

The Alpha handoff freezes the hosted compiler, CLI, standard-library, runtime,
LSP, and diagnostic contracts documented by ADR-0045 and the Alpha language
documents. Phase 17 may consume these contracts but must not implement a
second semantic analyzer in an extension. Raw runtime bridge privacy remains
owned by ADR-0047 and Phase 20; bare-metal service substitutions remain owned
by a future embedded phase.

## Phase 19 invariant

- [x] A real Actus application can be checked, built, run, tested, formatted,
      and reproduced through documented commands.
- [x] Struct ownership, layout, cleanup, and ABI rules are explicit.
- [x] Public standard-library failures are typed and platform-independent.
- [x] No implicit conversion, hidden allocation, or undocumented ownership
      transition is required by accepted Alpha programs.
- [x] Phase 17 can consume versioned compiler contracts without becoming a
      second compiler.

## Completion criteria

- [x] Gates 19.0 through 19.6 are complete.
- [x] Phase 16 Gate 6 is complete or all deferrals are explicitly accepted.
- [x] Remaining Phase 11 struct items are complete or explicitly deferred.
- [x] At least two real Actus applications pass native execution checks.
- [x] Supported host quality evidence is recorded.
- [x] Alpha compatibility and diagnostic policies are published.
- [x] A clean checkout reproduces the documented acceptance report.
