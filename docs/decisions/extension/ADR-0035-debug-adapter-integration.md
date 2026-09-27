# ADR-0035: Debug Adapter Integration

- Status: Proposed
- Date: 2026-09-27
- Scope: Actus source debugging through VS Code DAP clients

## Context

Source-level debugging is valuable for host binaries, simulators, and embedded
targets, but it is only trustworthy when compiler line tables, local-variable
locations, ABI descriptions, packed layouts, and target debug providers agree.
A launcher that merely starts GDB or LLDB is not an Actus debugger contract.

## Decision

Actus will integrate with the Debug Adapter Protocol through a thin adapter
and generated launch configuration. Existing debuggers and probe providers
remain responsible for transport and machine control. The compiler is
responsible for deterministic debug metadata, and the target manager selects a
provider only when its capabilities match the build artifact.

The first milestone is host/simulator debugging with source breakpoints,
step control, stack frames, locals, and exit status. Embedded debugging is
enabled only after the selected provider passes its capability checks.

## Debug metadata contract

The compiler must emit, where supported:

- source file identities and line/column mappings;
- verb and inline-call boundaries;
- local and parameter locations across the ABI;
- struct, enum, `Result`, `Option` niche, and `pack` layouts;
- pointer-sized `abs`/`ins` reference descriptions;
- role metadata as a non-semantic debug attribute; and
- optimized-out and unavailable states rather than fabricated values.

Role labels are descriptive. The debugger must never imply that inspecting a
value grants ownership or changes Actus semantic state.

## Adapter behavior

The adapter translates CodeLens and launch requests into DAP operations,
preserves cancellation, forwards structured stderr separately from protocol
traffic, and records the selected artifact hash and target profile. It must
support breakpoints before launch, conditional breakpoints only when the
backend supports them, and clear reporting when a requested feature is not
available.

The extension must not parse debugger output with fragile regular expressions
when a DAP field exists. Provider-specific behavior belongs behind the adapter
boundary.

## Safety and failure behavior

Debug actions are opt-in and target-scoped. Attach, reset, flash, and erase
operations require explicit provider capabilities and confirmation. A failed
debug session must leave no stale process or port handle owned by the
extension. Logs are redacted according to the provider contract and never
sent to a webview as executable markup.

## Verification

- [ ] Host integration tests verify breakpoints, stepping, locals, and exit
      status on a stable fixture.
- [ ] Debug metadata tests verify source spans and `pack` bit offsets.
- [ ] ABI tests cover struct returns, wide integers, floats, `Void`, and
      pointer-sized references.
- [ ] Negative tests cover unavailable locals and optimized-out values.
- [ ] Provider tests cover cancellation, timeout, attach failure, and cleanup.
- [ ] Cross-platform smoke tests run only where the selected provider exists.

## Non-goals

This ADR does not implement a debugger engine, promise universal DWARF
support, or make a host debugger suitable for a physical board without a
validated provider.
