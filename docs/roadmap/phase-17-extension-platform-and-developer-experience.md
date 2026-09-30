# Phase 17: Extension Platform and Developer Experience

Phase 17 turns Actus tooling from syntax support into a compiler-backed
engineering environment. It starts only after Phase 16 has completed the
standard library, runtime, portability, and baseline tooling acceptance
criteria.

The VS Code extension is the first consumer, but the contracts are designed
for every LSP/DAP client. The compiler, semantic analyzer, target model, and
header exporter remain authoritative. The extension renders their results and
starts explicitly authorized workflows; it does not become a second compiler.

The architectural decisions for this phase are grouped under
[`docs/decisions/extension/`](../decisions/extension/README.md).

## Scope and sequencing

- [ ] Keep Phase 16 limited to standard-library completion and baseline
      compiler/tooling parity.
- [ ] Freeze the extension protocol version and compatibility policy.
- [ ] Implement read-only compiler-backed inspection before mutating or device
      workflows.
- [ ] Add workflow actions only after source/document-version validation is
      available.
- [ ] Enable physical hardware actions only through declared target/provider
      capabilities and explicit confirmation.
- [ ] Keep all seven capabilities independently disableable.

## Current phase status

Phase 17 is **paused / deferred** after the compiler-backed LSP core gates.
Gates 17.0.1 through 17.0.10 are complete and covered by the compiler and
protocol test suites. The remaining shared-platform work is intentionally
preserved for a later continuation:

- Workspace and package integration remains open.
- Versioned contracts, unsaved-source synchronization, and the Actus command
  surface remain open.
- Gates 17.1 through 17.8 remain future feature phases.

This status is not a completion claim. The extension remains usable in its
current tested state, while compiler-core and release-readiness work takes
priority.

## Gate 17.0: Shared Extension Platform

Gate 17.0 is the compiler-backed LSP foundation. The server must provide a
Rust-analyzer-grade semantic platform for Actus rather than a collection of
independent text features. The compiler remains authoritative for parsing,
types, ownership, modules, targets, layouts, and diagnostics; the LSP is an
adapter and query surface over that information.

### Gate 17.0.1: Protocol lifecycle and versioned contracts

See [ADR-0048](../decisions/extension/ADR-0048-lsp-execution-cancellation-and-bounded-responses.md).

- [x] Establish the initial Actus initialize contract with protocol version,
      schema version, selected target, and explicit result-state vocabulary.
- [x] Return a structured JSON-RPC invalid-params error for request payloads
      that fail the server's typed parameter contract without terminating the
      LSP process.
- [x] Implement complete `initialize`, `initialized`, `shutdown`, and `exit`
      lifecycle behavior with deterministic error responses.
- [x] Add protocol fixtures for initialize metadata, capability negotiation,
      invalid params, unknown methods, and lifecycle errors.
- [x] Attach protocol, schema, compiler, target, and current document
      URI/version metadata to JSON-RPC responses and structured errors.
- [x] Define request identifiers, response correlation, notification handling,
      and structured JSON-RPC error contracts.
- [x] Add cancellation and progress handling for every potentially expensive
      request; canceled work must not publish stale results.
- [x] Define capability negotiation for compiler, server, client, and target
      versions, including graceful degradation for older clients.
- [x] Add versioned response metadata containing compiler version, protocol
      schema version, target profile, document URI, and document version.
- [x] Define explicit `available`, `stale`, `partial`, `unsupported`, and
      `invalid` result states instead of silently returning empty data.
- [x] Add JSON fixtures for successful, degraded, rejected, and canceled
      protocol exchanges.

### Gate 17.0.2: Incremental workspace and semantic model

- [x] Build a versioned workspace model for open documents, package manifests,
      module facades, siblings, imports, and target configuration.
- [x] Preserve full, incremental, and close overlay updates with strict
      document-version validation.
- [x] Recompute only affected modules and dependents after an overlay change;
      unrelated documents must retain valid cached results.
- [x] Add deterministic module and symbol indexes for package-local and
      workspace-wide queries.
- [x] Make every compiler-backed query cancelable and invalidate results when
      a newer document version supersedes the request.
- [x] Bound overlay size, workspace traversal, result count, and cache memory;
      report bounded or partial results explicitly.
- [x] Test unsaved edits, deleted files, renamed modules, facade changes,
      dependency changes, and concurrent document updates.

### Gate 17.0.3: Navigation and symbol intelligence

- [x] Implement definition, declaration, type definition, implementation,
      references, document symbols, workspace symbols, and call hierarchy.
- [x] Resolve local symbols, imported facade exports, private sibling symbols,
      generic instantiations, pack fields, and runtime-backed declarations.
- [x] Preserve exact source spans and normalized cross-platform URIs for every
      navigation result.
- [x] Index symbols incrementally and return deterministic ordering with
      bounded result sets.
- [x] Add accepted and rejected tests for private visibility, stale overlays,
      duplicate symbols, ambiguous imports, and missing definitions.

### Gate 17.0.4: Code intelligence and type information

- [x] Implement context-aware completion for declarations, fields, modules,
      facade exports, operators, ownership roles, and target-specific symbols.
- [x] Add completion item resolution with documentation, detail, source span,
      ownership contract, and replacement edits.
- [x] Implement signature help with parameter ownership, generic parameters,
      result types, error variants, and active-parameter tracking.
- [x] Expose inferred types, ownership states, borrow states, pack layouts,
      array capacities, buffer element types, and target-dependent types.
- [x] Keep completion and hover results synchronized with the same semantic
      snapshot used by diagnostics and definition queries.
- [x] Add negative coverage for incomplete syntax, invalid ownership, private
      declarations, unsupported targets, and stale semantic snapshots.

### Gate 17.0.5: Editing, refactoring, and source workflows

- [x] Implement rename and prepare-rename with compiler-validated symbol
      identity across modules, facades, overlays, and generated references.
- [x] Add compiler-backed source actions for formatting, organize-imports,
      and safe module-facade edits without duplicating compiler semantic rules
      in the LSP; semantic quick fixes remain gated on compiler-provided edits.
- [x] Add document formatting and range formatting with stable, idempotent
      output and explicit refusal for invalid or incomplete source.
- [x] Add organize-imports and module-facade edits with atomic workspace edits.
- [x] Implement CodeLens for supported entry points and executable verbs only
      after source version and target capability validation.
- [x] Display stdout, stderr, exit code, cancellation, and diagnostics for
      source workflows through structured results.
- [x] Test invalid source, unsaved edits, cross-module rename, path validation,
      ambiguous declarations, and unsupported targets/providers.

### Gate 17.0.6: Diagnostics and recovery

- [x] Unify lexer, parser, semantic, ownership, target, and code-generation
      diagnostics behind stable codes, ranges, severity, and related data.
- [x] Add deterministic recovery for incomplete source so editor queries remain
      useful without hiding real compiler errors.
- [x] Publish diagnostics only for the current document version and clear them
      deterministically on close or successful replacement.
- [x] Include actionable related locations, notes, ownership transitions, and
      suggested fixes where the compiler provides them.
- [x] Never let the semantic analyzer or code generator write directly to the
      terminal; LSP rendering remains an independent adapter concern.
- [x] Test malformed tokens, incomplete declarations, invalid imports, type
      errors, ownership failures, target filtering, and recovery boundaries.

### Gate 17.0.7: Actus semantic intelligence

- [x] Expose `erg`, `abs`, `dat`, and `ins` roles without changing ownership
      semantics or inventing editor-only states.
- [x] Render active, frozen, suspended, moved, and dropped transitions from
      compiler ownership records, including cleanup and reborrow edges.
- [x] Explain `Array` and `Buffer` element types, dynamic-index bounds,
      checked casts, and deterministic trap behavior.
- [x] Expose `pack` backing types, field offsets, widths, masks, ranges,
      reserved fields, and target data-model information.
- [x] Show target-filtered declarations and explain unavailable host,
      freestanding, board, and runtime capabilities.
- [x] Keep all displayed facts traceable to compiler source spans and semantic
      snapshots; the extension must not infer or repair compiler behavior.
- [x] Add fixtures for ownership moves, loans, cleanup, arenas, packs, arrays,
      buffers, generics, target declarations, and error propagation.

### Gate 17.0.8: Performance, cancellation, and bounded behavior

- [x] Introduce a query-oriented cache boundary so repeated requests reuse
      parsed and analyzed data without stale cross-version results.
- [x] Propagate cancellation through parsing, module aggregation, semantic
      analysis, indexing, formatting, and result serialization.
- [x] Debounce only at the editor/client boundary; compiler queries themselves
      must remain deterministic and independently testable.
- [x] Enforce bounded workspace traversal, symbol indexing, diagnostics,
      completion items, hover payloads, and serialized responses.
- [x] Add large-workspace benchmarks and regression thresholds for latency,
      memory, cancellation completion, and cache invalidation.
- [x] Verify that partial or timed-out results are explicitly labeled and never
      presented as complete semantic facts.

### Gate 17.0.9: Cross-platform and protocol correctness

- [x] Normalize file URIs, paths, and source locations correctly on Linux,
      macOS, and Windows, including UTF-8/UTF-16 editor positions.
- [x] Test CRLF/LF documents, Unicode identifiers and comments, non-ASCII
      paths, drive prefixes, UNC paths, and case-sensitive module resolution.
- [x] Run the complete LSP contract suite on all supported host platforms.
- [x] Verify that target-specific features never silently fall back to host
      runtime behavior.
- [x] Add protocol compatibility fixtures for old schema versions, missing
      optional capabilities, malformed client payloads, and unknown requests.

### Gate 17.0.10: Production verification

- [x] Add unit, semantic, native, protocol, and end-to-end tests for every
      Gate 17.0 contract and its negative cases.
- [x] Add deterministic JSON fixtures and snapshot review for all public
      response schemas.
- [x] Verify no private compiler declaration, raw bridge, backend type, or
      implementation-only field leaks through public LSP data.
- [x] Run formatting, compilation, Clippy, full tests, source limits, diff
      checks, and standard-library conformance checks.
- [x] Record Linux, macOS, and Windows CI evidence for the complete LSP suite.
- [x] Update the relevant extension ADRs, language reference, protocol
      documentation, and compatibility policy only after tests are green.

Gate 17.0.10 evidence is split by responsibility: `src/lsp/position.rs` and
`src/lsp/uri.rs` provide unit-level source-location and URI coverage;
`tests/lsp/semantic.rs`, `tests/lsp/semantic_states.rs`, and
`tests/lsp/platform.rs` cover compiler-backed semantic and cross-platform
behavior; `tests/lsp/production.rs` locks public response families and
diagnostics notifications to reviewed JSON contract snapshots;
`tests/lsp/protocol.rs` covers compatibility and lifecycle failures; and
`tests/lsp_workflow.rs` covers process-level stdio execution and navigation.
The CI matrix runs the isolated 64-test LSP suite and the complete workspace
suite on Linux, macOS, and Windows.

### Workspace and package integration

- [ ] Define extension package identity, supported VS Code versions, and
      minimum `actus`/LSP versions.
- [ ] Add configuration discovery for compiler path, workspace root, target
      profile, and feature capabilities.
- [ ] Keep the extension functional when optional providers are unavailable.
- [ ] Add a single Activity Bar container with views registered by capability.
- [ ] Define view lifecycle, disposal, refresh, and workspace trust behavior.
- [ ] Ensure webviews use local packaged assets and a restrictive CSP.
- [ ] Verify clean VSIX packaging contains no development-only paths.

### Versioned contracts

- [ ] Define schemas for ownership graphs, pack layouts, allocation effects,
      target capabilities, and generated export manifests.
- [ ] Include compiler version, schema version, target profile, document URI,
      and document version in every response.
- [ ] Define capability negotiation for old servers and partial clients.
- [ ] Define structured unavailable, stale, partial, and invalid states.
- [ ] Add JSON fixtures and compatibility tests before UI implementation.
- [ ] Ensure all source locations use the canonical UTF-8/UTF-16 adapter.

### Unsaved source and synchronization

- [ ] Propagate `didOpen`, full `didChange`, incremental `didChange`, and
      `didClose` overlays to every feature request.
- [ ] Recompute sibling/facade context from the current in-memory package.
- [ ] Reject stale actions when the document version changed after resolution.
- [ ] Add cancellation and bounded-result behavior for large workspaces.

### Actus command surface

- [ ] Define typed extension commands for `actus new`, `check`, `build`,
      `run`, `test`, `fmt`, `watch`, and `lock` without duplicating CLI logic.
- [ ] Resolve workspace root, manifest, compiler path, target, profile, and
      feature flags from the active workspace configuration.
- [ ] Expose commands through the Command Palette and an Activity Bar command
      view with stable labels, keyboard navigation, and disabled states.
- [ ] Stream command progress and diagnostics into Output, Problems, and the
      status view while preserving the exact process exit status.
- [ ] Support cancellation, concurrent-command exclusion, and deterministic
      cleanup when a workspace closes or trust is revoked.
- [ ] Reject command execution in untrusted workspaces and report missing
      compiler, invalid manifests, spawn failures, and non-zero exits as
      structured user-facing states.
- [ ] Test command construction, target/profile propagation, trust gating,
      cancellation, failure rendering, and multi-root workspace selection.

## Gate 17.1: Memory and Ownership Graph

See [ADR-0032](../decisions/extension/ADR-0032-memory-ownership-graph.md).

- [ ] Add `actus/memoryGraph` request and response schema.
- [ ] Export declarations, parameters, owners, views, loans, moves, arena
      roots, provenance edges, and cleanup edges with source spans.
- [ ] Represent `Active`, `Frozen`, `Suspended`, `Moved`, and `Dropped`
      transitions without inventing new semantic states.
- [ ] Distinguish current loans from historical loans and planned cleanup from
      completed cleanup.
- [ ] Add summary and full-detail modes with bounded node/edge counts.
- [ ] Render role labels and accessible non-color cues for `erg`, `abs`,
      `dat`, and `ins`.
- [ ] Navigate from every node, edge, and diagnostic to declaration and use.
- [ ] Render partial graphs with explicit recovery/exclusion explanations.
- [ ] Test moves, frozen owners, suspended loans, arena provenance, `?`,
      return/break/continue cleanup, and unsaved sibling changes.

## Gate 17.2: Pack and MMIO Layout Viewer

See [ADR-0033](../decisions/extension/ADR-0033-pack-mmio-layout-viewer.md).

- [ ] Add `actus/packLayout` request backed by resolved semantic layout data.
- [ ] Show backing type, capacity, byte width, endianness, target data model,
      density, padding, and declaration location.
- [ ] Show every field's name, role, type, offset, width, range, mask, default,
      reserved status, and source span.
- [ ] Render bit numbering and byte lanes without conflating bit order and
      byte order.
- [ ] Make the first release strictly read-only and prevent hardware writes.
- [ ] Add local value encoding preview with compiler-validated masks.
- [ ] Add “copy as Actus literal” only for validated deterministic values.
- [ ] Test little/big endian, `u8` through `u128`, reserved fields, overlap,
      capacity overflow, and incomplete declarations.

## Gate 17.3: CodeLens and Source Workflows

See [ADR-0036](../decisions/extension/ADR-0036-code-lens-workflows.md).

- [ ] Add CodeLens for supported entry points and executable verbs.
- [ ] Add `Run`, `Run Test`, `Debug`, and `Debug Test` actions with capability
      checks and structured arguments.
- [ ] Add pack `Inspect Layout` and read-only size/density summaries.
- [ ] Bind every lens to a document version and source symbol identity.
- [ ] Refresh lenses after open/change/close and hide ambiguous declarations.
- [ ] Show stdout, stderr, exit code, cancellation, and compiler diagnostics.
- [ ] Test invalid source, unsaved edits, path validation, and unsupported
      targets/providers.

## Gate 17.4: Target and Board Manager

See [ADR-0034](../decisions/extension/ADR-0034-target-board-manager.md).

- [ ] Define target-profile schema for triple, ABI, pointer width, endianness,
      features, linker inputs, and standard-library capabilities.
- [ ] Define board-profile schema for probe filters, reset policy, flash,
      debug, simulation providers, timeouts, and artifact kinds.
- [ ] Discover provider capabilities instead of assuming installed tools.
- [ ] Add profile validation and explain every rejected capability.
- [ ] Add deterministic build, dry-run, flash, simulate, and cancel actions.
- [ ] Require confirmation for attach, reset, erase, flash, and other device
      mutations, naming target, probe, artifact, and destructive scope.
- [ ] Keep commands configuration-driven and reject source-controlled command
      injection.
- [ ] Test missing tools, timeout, cancellation, wrong probe, failed reset,
      and artifact-hash mismatch.

## Gate 17.5: Debug Adapter Integration

See [ADR-0035](../decisions/extension/ADR-0035-debug-adapter-integration.md).

- [ ] Define the Actus-to-DAP launch and capability mapping.
- [ ] Emit and verify source files, line mappings, verb boundaries, locals,
      parameters, and inline-call metadata.
- [ ] Verify debug layouts for structs, enums, `Result`, `Option` niches,
      `pack` fields, wide integers, floats, `Void`, and references.
- [ ] Display role labels as metadata without changing ownership semantics.
- [ ] Support source breakpoints, stepping, stack frames, locals, and exit
      status for host/simulator fixtures.
- [ ] Report unavailable or optimized-out values honestly.
- [ ] Add provider cleanup, cancellation, attach failure, and timeout tests.
- [ ] Gate physical hardware debugging on a validated provider profile.

## Gate 17.6: Zero-Allocation and Real-Time Safety Analysis

See [ADR-0037](../decisions/extension/ADR-0037-zero-allocation-analysis.md).

- [ ] Define compiler allocation effects: `NoAlloc`, `MayAlloc`, and
      `RequiresHostService`.
- [ ] Compute direct and transitive effects through calls, generics, role
      dispatch, target filtering, and error paths.
- [ ] Track source spans and causal call edges for every effect.
- [ ] Add strict contexts that reject unknown or unsafe effects rather than
      treating them as safe.
- [ ] Classify bounded arena placement, pack operations, and bulk reset only
      when their contracts are proven.
- [ ] Expose `actus/allocationEffects` to LSP with target and version data.
- [ ] Render explanations and navigation in the extension without claiming a
      proof from incomplete analysis.
- [ ] Test host `std::fs`, freestanding arena/pack code, `?` paths, generic
      calls, and target-specific effects.

## Gate 17.7: Deterministic C Header Export

See [ADR-0038](../decisions/extension/ADR-0038-c-header-export.md).

- [ ] Add compiler-owned header export command and library entry point.
- [ ] Require an explicit package root, target profile, export policy, and
      output path.
- [ ] Export only validated public C-ABI declarations and approved concrete
      generic specializations.
- [ ] Emit fixed-width pack constants and masks instead of implementation-
      defined C bitfields.
- [ ] Reject unsupported roles, `Result`/`Option` representations, recursive
      references, ambiguous widths, and unfrozen ABIs.
- [ ] Emit deterministic header ordering, guards, formatting, and manifest
      fingerprints.
- [ ] Add preview, atomic output, cancellation, and path validation in the
      extension action.
- [ ] Verify repeated generation is byte-identical and target-aware.

## Gate 17.8: Cross-Platform Release and Accessibility

- [ ] Run compiler/LSP contract tests on Linux, macOS, and Windows.
- [ ] Build and package the VSIX from a clean checkout on all supported hosts.
- [ ] Verify standard dark/light themes and high-contrast mode.
- [ ] Verify keyboard navigation, screen-reader labels, focus order, and
      non-color ownership/bitfield cues.
- [ ] Verify webview CSP, local-resource restrictions, and workspace trust.
- [ ] Verify no feature silently invokes host runtime or allocator services on
      freestanding targets.
- [ ] Document provider installation, failure recovery, and supported profiles.
- [ ] Record schema compatibility and migration policy.

## Phase 17 invariants

- [ ] Every displayed fact comes from a compiler, LSP, DAP, or declared target
      provider contract.
- [ ] The extension never duplicates parser, ownership, type, layout, or ABI
      logic.
- [ ] Stale, partial, unsupported, and unknown states are explicit.
- [ ] Read-only inspection cannot mutate source, artifacts, or devices.
- [ ] Device actions require an explicit configured capability and confirmation.
- [ ] Target-specific functionality never silently falls back to host runtime.
- [ ] The extension remains useful when optional providers are absent.

## Phase 17 completion criteria

- [ ] All seven feature ADRs have implemented contracts and passing fixtures.
- [ ] The VS Code Activity Bar and views are packaged from a clean workspace.
- [ ] At least one host workflow and one freestanding/embedded simulation
      workflow are reproducible from documented profiles.
- [ ] Compiler, LSP, Tree-sitter, VS Code, and DAP compatibility checks pass.
- [ ] Security, accessibility, cancellation, and stale-overlay tests pass.
- [ ] Public documentation and generated manifests match the implementation.
