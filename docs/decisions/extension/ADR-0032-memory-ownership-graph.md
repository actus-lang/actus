# ADR-0032: Memory and Ownership Graph Inspector

- Status: Proposed
- Date: 2026-09-27
- Scope: Actus LSP and VS Code extension developer experience

## Context

Actus makes ownership state explicit through `erg`, `abs`, `dat`, and `ins`.
The compiler already reasons about active owners, frozen views, suspended
loans, moves, provenance, scope cleanup, and early-return unwinding. A plain
diagnostic explains why a program is rejected, but it does not show the state
transitions that led to the diagnostic.

An editor must not reconstruct this state from syntax, regular expressions, or
diagnostic wording. Such a second ownership model would drift from the
compiler and could display a graph that is not the graph used for acceptance.

## Decision

The compiler/LSP will expose an optional, versioned `actus/memoryGraph`
request. It returns a read-only semantic snapshot for the function and source
range selected by the client. The VS Code extension renders that snapshot in
an Activity Bar webview or an inline companion view.

The compiler remains authoritative. The extension is a renderer and
navigation layer only.

## Contract

The request contains:

- document URI and document version;
- selected byte range or LSP position;
- optional function or binding identifier; and
- requested detail level (`summary` or `full`).

The response contains:

- schema version and compiler build identity;
- source document version used for analysis;
- nodes for declarations, parameters, owners, views, loans, arena roots,
  moved values, and cleanup actions;
- directed edges for ownership, view origin, loan, move, provenance, call,
  and cleanup dependencies;
- state transitions with source spans and stable semantic state names;
- diagnostic references for rejected transitions; and
- a `complete` flag plus an explanatory reason when recovery made the graph
  partial.

Every node and edge that can be shown in the editor carries a source span.
The server must use the same UTF-8/UTF-16 position adapter as diagnostics and
definition results.

## State and rendering semantics

The canonical states are `Active`, `Frozen`, `Suspended`, `Moved`, and
`Dropped`, matching the semantic model. Role colors are presentation hints,
not semantic truth: `erg` is green, `abs` cyan, `dat` amber, and `ins`
violet in the official theme. Clients must remain readable in standard themes
by using labels, icons, and patterns in addition to color.

The view must distinguish:

- an owner from a non-owning view;
- a currently live loan from a historical loan;
- a move edge from a borrow edge;
- an arena provenance edge from an ordinary field edge; and
- planned cleanup from completed cleanup.

The graph is deterministic for identical source text, compiler version,
target profile, and document version. Node identifiers are stable within one
response but are not persisted as public ABI.

## Unsaved documents and failure behavior

The graph is computed from the LSP document overlay, including unsaved sibling
facade members. A stale response must be marked stale and must not silently
replace a newer view. Syntax or semantic recovery may return a partial graph,
but every omitted region must be reported.

Unsupported server versions, invalid requests, and unavailable semantic
snapshots produce a structured LSP error or an explicit unavailable state;
the extension never guesses ownership transitions.

## Security and performance

The request is read-only and cannot execute Actus code or mutate a device.
The server may cache snapshots by document version and semantic fingerprint.
Large functions must support summary mode and bounded graph payloads. Webview
content must use the extension's local, sanitized resources and must not load
remote scripts.

## Verification

- [ ] Schema fixtures cover all four roles and all ownership states.
- [ ] Tests cover moves, frozen-owner rejection, suspended loans, arena
      provenance, return unwinding, `?` unwinding, and bulk cleanup.
- [ ] LSP tests verify UTF-16 navigation from nodes and edges.
- [ ] Extension tests verify stale overlays and partial graphs.
- [ ] Accessibility tests verify keyboard navigation and non-color cues.

## Non-goals

This ADR does not add a new ownership rule, change compiler acceptance, or
promise a general-purpose heap visualizer. Runtime heap tracing is a separate
future capability and must not be implied by a static semantic graph.
