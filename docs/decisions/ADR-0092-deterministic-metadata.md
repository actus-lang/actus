# ADR-0092: Deterministic verb contracts

- Status: Accepted
- Date: 2026-10-29
- Scope: `meta deterministic`, effect-aware semantic validation, diagnostics,
  formatter, LSP, and role/generic dispatch

## Context

Deterministic systems code must make external sources of variation visible.
The existing effect contract identifies filesystem, network, entropy, blocking,
and allocation behavior, but callers still need a declaration that forbids the
effects that can change a result across executions.

## Decision

`meta deterministic` is a zero-argument declaration attribute. A deterministic
verb may not call a signature carrying `filesystem`, `network`, or `entropy`.
The check is performed during semantic analysis at the call site and produces
`E1124` for a forbidden effect.

The contract propagates through imported and external signatures, generic
specialization, and static/dynamic role dispatch. Role implementations must
match the role method's deterministic declaration exactly. The attribute does
not add runtime checks or imply that undeclared native behavior is safe;
effect declarations remain mandatory at native boundaries.

## Safety invariants

- Determinism is explicit and visible at the declaration and call site.
- No filesystem, network, or entropy dependency can enter a deterministic verb
  through a declared effect signature.
- `blocking` and `allocation` remain separate effect dimensions.
- Ownership, cleanup, ABI, and binary layout are unchanged.
- Diagnostics are stable and formatter/LSP output preserves the declaration.

## Consequences

The compiler can reject the most important external nondeterministic sources
without pretending to prove arbitrary native code. Future clock, scheduler,
iteration-order, and platform-capability contracts must be represented by
explicit effects or capabilities before they are included in this guarantee.
