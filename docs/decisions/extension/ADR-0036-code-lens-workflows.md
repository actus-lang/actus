# ADR-0036: CodeLens Workflows for Actus Sources

- Status: Proposed
- Date: 2026-09-27
- Scope: LSP CodeLens and VS Code source actions

## Context

Actus developers should be able to run a verb, test a fixture, or inspect a
packed layout from the declaration they are reading. CodeLens is a standard
LSP feature and can provide this ergonomics without embedding compiler logic
in the extension.

## Decision

The LSP will expose `textDocument/codeLens` results derived from the current
semantic snapshot. The extension renders and resolves those lenses through
standard LSP commands plus explicitly namespaced Actus commands.

Initial lenses are:

- `Run` for an executable entry point or supported verb fixture;
- `Debug` when the selected target and DAP provider support it;
- `Run Test` and `Debug Test` for recognized test declarations;
- `Inspect Layout` for a `pack` declaration; and
- a read-only pack summary showing size, density, and field count.

The server must not return an action when the declaration is incomplete,
unsupported, or ambiguous. A lens is tied to a source span and document
version; resolution revalidates that version before execution.

## Command contract

Commands carry structured arguments: URI, source span, symbol identity,
document version, target profile, and requested action. The extension asks the
compiler CLI or target manager to execute the action and displays stdout,
stderr, exit status, cancellation, and diagnostics in existing VS Code
surfaces. Command strings are never built by concatenating source text.

Run and test actions execute in a controlled build directory with explicit
environment and target configuration. Debug actions delegate to ADR-0035.
Layout inspection delegates to ADR-0033 and is always read-only.

## Invalidation and overlays

Lenses are recomputed after accepted `didOpen` and `didChange` updates. The
server must include a document version in resolved commands. If the file has
changed, resolution fails with a refresh response rather than running stale
source.

## Verification

- [ ] LSP tests cover verbs, tests, packs, invalid source, and unsaved edits.
- [ ] Integration tests verify run/test exit status and captured output.
- [ ] Tests verify absent lenses for unsupported targets and providers.
- [ ] UI tests cover keyboard access, theme contrast, and refresh behavior.
- [ ] Security tests reject path traversal and untrusted command arguments.

## Non-goals

This ADR does not add a test framework, alter `actus run` semantics, or make
CodeLens a replacement for the terminal CLI. It does not authorize automatic
flashing or device mutation.
