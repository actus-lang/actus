# ADR-0100: Deterministic floating-point exclusion

- Status: Accepted
- Date: 2026-11-03
- Scope: `meta deterministic`, semantic analysis, call boundaries, role
  contracts, diagnostics, and language documentation

## Context

Actus supports floating-point types for applications that explicitly need
them. Floating-point execution is nevertheless not a portable deterministic
contract: target instruction sets, rounding modes, NaN handling, external ABI
behavior, and compiler lowering can change the observable result. Avoiding
clock, filesystem, network, and entropy effects alone therefore does not make
a deterministic verb reproducible across supported targets.

## Decision

`meta deterministic` is an integer-oriented reproducibility contract. A
deterministic declaration must not declare or use `f32` or `f64`, including
values nested in generic or fixed-array types. The restriction covers:

- parameters and return types;
- local bindings and floating-point literals;
- floating-point expressions and casts;
- direct, imported, external, generic, and dynamic role calls;
- role methods and their implementations.

The compiler rejects a violation during semantic analysis. It does not insert
runtime checks, conversions, software floating-point helpers, or hidden
normalization. Floating-point code outside a deterministic call graph remains
valid and is governed by the ordinary target and backend contracts.

The restriction is transitive at signatures: a deterministic caller cannot
reach a callable whose parameter or return contract contains floating-point,
even when the callable is external, imported, generic-specialized, or reached
through dynamic dispatch. A role method marked deterministic requires every
implementation to preserve that declaration.

## Diagnostics

- `E1135` identifies floating-point use in deterministic code or at a
  deterministic call boundary.
- `E1136` identifies an implementation that does not preserve a role method's
  deterministic contract.

## Consequences

This keeps the language's floating-point capability available without making
`meta deterministic` weaker or dependent on backend-specific behavior. Users
who need deterministic decimal or sensor arithmetic must use integer or
explicit fixed-point representations inside the deterministic boundary.

Ownership, cleanup, ABI layout, and runtime representation are unchanged.
