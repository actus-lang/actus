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

## Gate 17.0: Shared Extension Platform

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
