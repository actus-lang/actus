# ADR-0045: Actus Alpha Core and Application Readiness

- Status: Proposed
- Date: 2026-09-29
- Scope: Language-core completion, standard-library acceptance, real application
  workflows, and Alpha compatibility policy

## Context

Actus now has a substantial language and compiler foundation. The compiler
supports explicit ownership roles, deterministic cleanup, generic dispatch,
packed layouts, bounded arrays, checked casts, Buffer indexing, lexical arenas,
native Cranelift lowering, relational operators, equality, remainder, logical
short-circuiting, bitwise operations, shifts, and cross-platform path handling.

The compiler and editor tooling have also reached a coordinated state. ADR-0044
is accepted, the LSP surface is synchronized with the compiler, Tree-sitter
has generated parser artifacts and corpus coverage, and the VS Code extension
recognizes the current operator surface.

The remaining risk is no longer the absence of isolated language features. The
risk is that the project could continue adding primitives without proving that
the complete system supports ordinary, reproducible Actus programs. Several
acceptance items remain in the existing Phase 16 Gate 6, and the original
struct roadmap still contains unresolved semantic and ABI contracts. These
items must be closed explicitly before Actus can claim Alpha-level application
readiness.

Phase 17, the extension platform, is intentionally not the next implementation
target. It depends on a stable compiler, standard-library, package, and
diagnostic contract. Extension features must consume those contracts rather
than define them.

## Decision

Actus will introduce a dedicated Alpha Core and Application Readiness program.
The program is tracked as Phase 19 and is governed by this ADR. It completes
the language and runtime contracts needed to write, build, test, and distribute
real Actus utilities before extension-platform implementation begins.

Phase 19 does not replace the existing Phase 16 roadmap. Phase 16 Gate 6
remains the authoritative checklist for standard-library, CLI, portability,
profile, lockfile, and workflow acceptance. Phase 19 coordinates those
remaining items with the unfinished struct contracts and adds the release and
compatibility criteria that are broader than the original Phase 16 scope.

### Objectives

Phase 19 has six objectives:

1. close or explicitly resolve every Phase 16 Gate 6 acceptance item;
2. complete the remaining struct ownership, visibility, representation, and
   public-ABI contracts from Phase 11;
3. prove the standard library through real applications rather than isolated
   declarations or syntax fixtures;
4. define a stable Alpha language subset, compatibility policy, and diagnostic
   contract;
5. establish deterministic build, test, artifact, and release evidence across
   supported host platforms; and
6. record the boundaries required before self-hosting or Phase 17 extension
   workflows begin.

### Sequencing and gates

Phase 19 will be implemented through the following ordered gates:

#### Gate 19.0: Baseline and contract inventory

- inventory every remaining unchecked item in Phase 11 and Phase 16 Gate 6;
- map each item to a compiler phase, runtime boundary, standard-library
  module, test family, and documentation record;
- identify contradictions between the README, manifesto, language documents,
  ADRs, and implementation;
- freeze the current accepted syntax and semantic surface as the Alpha baseline;
- define explicit deferrals instead of silently carrying unfinished work.

Gate 19.0 is complete only when every remaining item has one of three states:
implemented, scheduled in a later gate, or explicitly deferred with a reason.

#### Gate 19.1: Complete struct semantics

- define field visibility and access rules;
- define ownership roles for struct fields, including `erg`, `abs`, and `dat`;
- define copy, move, assignment, and partial-move behavior;
- define the rules for reference-bearing fields and reject escaping borrows;
- define packed and externally represented struct policies;
- define native layout and public C-ABI contracts where exposure is supported;
- add accepted and rejected semantic fixtures for every rule;
- document unsupported struct patterns with stable diagnostics.

Struct semantics must remain independent from backend details. The semantic
analyzer owns validity and ownership transitions; codegen consumes validated
layout and cleanup plans without repairing invalid programs.

#### Gate 19.2: Standard-library and real-application acceptance

- create a real console application using `std::io`;
- create a real file utility using `std::fs` and `std::path`;
- verify `actus check`, `actus build`, and `actus run` for both applications;
- verify stdout, stderr, and process exit codes;
- verify typed I/O errors and cleanup on success and failure paths;
- verify facade imports and sibling-module visibility from external consumers;
- add beginner-to-running-program documentation based on working commands;
- keep application fixtures under `tests/` and examples under `examples/`.

An application is not accepted because it parses. It must compile through the
native backend, execute, produce the expected output, return the expected exit
status, and exercise its documented failure paths.

#### Gate 19.3: CLI, package, and reproducible workflow

- complete `actus test`, including deterministic discovery and reporting;
- complete `actus fmt` and `actus fmt --check` behavior;
- verify `actus watch` and `actus watch --once` without stale diagnostics;
- verify debug and release profiles produce valid, deterministic artifacts;
- verify lockfile generation, validation, and stale-state rejection;
- verify package/module imports and facade resolution from clean workspaces;
- define artifact naming, output-directory, and failure-cleanup contracts;
- verify repeated builds produce identical outputs where the target contract
  promises determinism.

CLI behavior is part of the Alpha contract. Invalid command forms must fail
closed with stable diagnostics and must not leave misleading artifacts behind.

#### Gate 19.4: Runtime, portability, and failure matrix

- run the complete compiler and standard-library quality matrix on Linux,
  macOS, and Windows;
- cover host-native path handling, line endings, executable names, linker
  selection, and process status behavior;
- verify raw host/C statuses remain private to runtime bridges;
- verify deterministic cleanup for early return, `?`, `break`, `continue`,
  failed I/O, and failed native execution;
- verify no hidden host allocation is introduced in bounded or zero-allocation
  contracts;
- record platform-specific substitutions for deferred bare-metal services;
- retain negative tests for unsupported targets and unavailable host services.

A green compiler build alone is insufficient. Each portability claim requires
the relevant command output or native execution evidence on the affected
platform.

#### Gate 19.5: Alpha compatibility and documentation

- define the Alpha language edition and supported compiler invocation;
- classify changes as compatible, diagnostic-only, edition-gated, or breaking;
- define the stability policy for diagnostic codes and rendered diagnostics;
- document ownership roles and their call-site spelling;
- document standard-library error contracts and runtime boundaries;
- document supported targets and explicitly unsupported host services;
- ensure README, manifesto, language references, examples, ADRs, and roadmap
  agree with the implementation;
- add migration notes for any behavior that changed during Phase 19.

Documentation must describe implemented behavior only. It must not promise
implicit conversions, allocation guarantees, lifetime behavior, or portability
that the compiler and runtime do not actually enforce.

#### Gate 19.6: Alpha release candidate and handoff

- run all required Rust and Actus checks from a clean checkout;
- run the full positive and negative application matrix;
- verify deterministic lockfile and artifact state twice from clean builds;
- publish a machine-readable acceptance report;
- record all remaining deferrals and their intended future phase;
- mark Phase 19 complete only when the Alpha completion criteria are met;
- hand Phase 17 only the versioned compiler/LSP contracts it is allowed to
  consume.

The handoff must be a documented boundary. Phase 17 must not begin by
re-implementing compiler semantics inside an extension or by depending on
unstable private output.

## Architectural contracts

### Compiler authority

The compiler remains the sole authority for syntax acceptance, type validity,
ownership transitions, cleanup planning, target filtering, and native ABI
selection. The standard library may expose typed wrappers, but it may not hide
ownership transfer, allocation, or raw host failure semantics.

### Ownership and cleanup

- `erg` remains the mutable active owner;
- `abs` remains a non-owning read-only view;
- `dat` remains a terminal ownership transfer;
- `ins` remains an exclusive call-scope loan restored to the caller;
- early return, `?`, `break`, and `continue` must preserve deterministic LIFO
  cleanup and loan restoration;
- a struct field must not silently change the ownership state of its containing
  value.

### Runtime and ABI boundaries

Runtime bridges must expose explicit status contracts and typed Actus results.
Raw C/POSIX or platform-specific statuses must not leak through public
standard-library APIs. Aggregate returns, buffers, arrays, packs, and struct
fields must use the documented ABI without hidden temporary ownership or
allocation.

### Determinism

The following must be deterministic where the target contract permits it:

- diagnostics and their ordering;
- test discovery and reporting;
- cleanup order;
- lockfile serialization;
- build graph resolution;
- generated artifact contents;
- package and module facade resolution.

If a host or linker makes a behavior inherently platform-specific, the
variation must be represented explicitly in the target contract and tested on
the affected platform.

## Alpha completion criteria

Phase 19 may be marked complete only when:

1. Phase 16 Gate 6 is complete or every deferral is documented and accepted;
2. Phase 11 struct semantics have explicit rules, diagnostics, tests, and
   public documentation;
3. at least two real Actus applications build and run successfully;
4. positive and negative tests cover the standard-library and CLI contracts;
5. Linux, macOS, and Windows quality evidence is recorded;
6. debug/release artifacts and lockfiles are deterministic under their stated
   contracts;
7. the Alpha compatibility and diagnostic policy is published;
8. no undocumented ownership, allocator, ABI, or host-service behavior is
   required by the accepted examples; and
9. the repository is ready for Phase 17 without making the extension a second
   compiler or semantic analyzer.

## Non-goals

Phase 19 does not include:

- the Activity Bar, ownership graph inspector, pack viewer, target manager,
  debug adapter, CodeLens, or other Phase 17 product features;
- self-hosting the compiler;
- advanced concurrency or task-transfer semantics;
- a tracing garbage collector or implicit heap management;
- implicit numeric, text, ownership, or truthiness conversions;
- replacing the compiler with editor-side semantic logic;
- claiming performance, portability, or safety properties without executable
  evidence;
- stabilizing APIs that are explicitly marked experimental or deferred.

## Consequences

This decision slows the addition of isolated syntax features in favor of
completing the contracts that make existing features usable together. It
creates a larger acceptance surface: real applications, platform behavior,
packaging, diagnostics, and documentation must all remain green.

The benefit is a defensible Alpha boundary. Users will be able to learn from
working programs, rely on explicit ownership and error behavior, and reproduce
builds across supported hosts. Future tooling and self-hosting work will have
versioned contracts instead of reverse-engineering unfinished implementation
details.

## Open questions for the implementation roadmap

The Phase 19 roadmap must resolve these questions before implementation of the
corresponding gate:

- Which struct visibility model is supported in Alpha: module-private only,
  public fields, or an explicit public-field marker?
- Which struct reference fields, if any, are permitted before a general lifetime
  model exists?
- Which host services belong in the Alpha standard library and which remain
  target-provider responsibilities?
- What exact artifact reproducibility guarantee is made for each supported
  target and linker family?
- Which diagnostics are stable for the Alpha edition, and how are future
  diagnostic wording changes versioned?

These questions are design inputs, not permission to add implicit behavior or
to bypass the compiler's ownership and target contracts.
