# ADR-0094: Statically checked realtime contracts

- Status: Accepted
- Date: 2026-10-29
- Scope: `meta realtime`, semantic restrictions, call-graph propagation, and diagnostics

## Decision

Actus supports `meta realtime` as a compile-time safety contract for verbs
whose execution path must remain bounded and free from host-dependent work.
The attribute does not optimize code, add runtime scheduling, or grant any
capability. It only makes the compiler reject source that cannot satisfy the
realtime contract.

The compiler rejects every call carrying a registered effect from a realtime
verb: `blocking`, `clock`, `filesystem`, `network`, `allocation`, and
`entropy`. The rule follows imported signatures, facades, generic calls,
external declarations, and role dispatch.

An unbounded `loop` is rejected. A range loop and an array loop remain valid
because their iteration source is statically bounded by the language model.
Floating-point literals and operations are rejected in realtime source.
Floating-point parameter, return, local, and call-result types are rejected as
well. Direct self-recursion is rejected because it has no statically declared
depth bound. The existing integer-only and ownership rules remain unchanged.

Realtime is part of role method contracts. If a role method is realtime, every
implementation must explicitly preserve that contract before it can be used
through dynamic or generic dispatch.

## Boundaries

This ADR deliberately does not claim a complete WCET proof. The compiler does
not yet infer machine-level instruction cost, cache behavior, interrupt
latency, recursion depth, or bounds hidden inside arbitrary native code.
Native declarations must expose their effects, and unsupported proof cases are
rejected by the existing signature rules rather than silently accepted.

`meta realtime` does not change ownership, cleanup, ABI layout, serialization,
or runtime behavior. It is an additional checked contract and cannot be
disabled with `meta limitless`.

## Evidence

The implementation includes parser, duplicate-metadata, role-contract,
effect-rejection, unbounded-loop, floating-point, formatter, and LSP fixtures.
Diagnostics are stable: E1125 for forbidden effects, E1126 for unbounded
loops, E1127 for floating point, E1128 for a missing role realtime contract,
and E1129 for unbounded direct recursion.
