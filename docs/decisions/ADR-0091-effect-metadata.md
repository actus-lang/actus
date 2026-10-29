# ADR-0091: Explicit effect metadata

- Status: Accepted
- Date: 2026-10-29
- Scope: `meta effect(...)`, effect declarations, call-graph propagation,
  diagnostics, tooling, and target-specific effect policy

## Context

Actus code must make blocking I/O, filesystem access, allocation, entropy, and
other operational boundaries visible. Today those properties are enforced only
by individual APIs and review. A compiler-recognized effect contract can make
the boundary checkable without hiding ownership, adding runtime reflection, or
turning a performance hint into a safety guarantee.

## Decision

Introduce a declaration-level form:

```actus
meta effect("filesystem")
verb load_manifest(abs path: Path) -> Result[Manifest, IoError] { ... }
```

The accepted effect vocabulary is intentionally closed and compiler-registered:
`blocking`, `clock`, `filesystem`, `network`,
`allocation`, and `entropy`. An unknown effect name is a compile-time error;
there is no user-defined escape hatch and no wildcard effect.

Effects are properties of a verb signature. A call to an effectful verb
propagates the declared effects to its caller's checked call graph. A caller
may explicitly declare compatible effects, but a declaration must not erase,
rename, or under-report a callee effect. Native declarations require the same
explicit contract at the ABI boundary.

Role methods are effect-bearing interface contracts. A role method declares an
upper bound for implementations: every `perform` implementation must declare
a subset of the role method's closed effect set. A pure implementation may
therefore satisfy an effect-bearing role, but an implementation may not add an
effect that the role did not declare. The role method's signature remains the
effect boundary used by dynamic dispatch and generic role-bound calls, while a
direct concrete call retains the implementation signature. This prevents
implementation-only effects from being hidden while allowing one role to have
both hosted and in-memory implementations.

Target filtering is effect-neutral: target selection may remove a declaration
because of its target metadata, but it must never remove or rewrite effects on
a declaration that remains. The accepted contract does not define a restricted
context such as `meta realtime`; a future restricted-context ADR may consume
these effect signatures without changing their meaning.

## Safety invariants

- Effects are compile-time contracts with no implicit runtime representation.
- Ownership, cleanup, ABI layout, and return roles remain unchanged.
- An effect cannot authorize an operation; it only describes an already
  declared capability or operational property.
- Callers cannot suppress effects with aliases, generics, facades, or external
  declarations.
- A role method must declare every effect permitted to any implementation;
  implementations may only declare subsets of that contract.
- Dynamic and generic role calls use the role's conservative effect contract;
  direct concrete calls use the concrete implementation's effects.
- The compiler must reject under-declared callers deterministically.
- Effect checking must not infer hidden allocation or filesystem behavior from
  names; only registered signatures and native contracts provide evidence.

The standard logical-region boundary follows the same rule. Opening a region
or remapping its resident window has an explicit `allocation` contract because
the hosted runtime creates replacement resident storage. Checked reads,
writes, publication, cancellation, lifecycle close, metadata inspection, and
pinning remain effect-free because they operate on already-owned bounded
storage and do not perform external I/O or allocate replacement storage.

## Required implementation evidence

Before acceptance, the implementation must include positive and negative
fixtures for direct calls, imported calls, nested facades, generic calls,
recursive call graphs, dynamic dispatch, external declarations, target
filtering, formatter output, LSP metadata, and stable diagnostics. The strict
suite must prove that effect metadata does not alter ownership or ABI behavior.
