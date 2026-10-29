# ADR-0091: Explicit effect metadata

- Status: Proposed
- Date: 2026-10-29
- Scope: `meta effect(...)`, effect declarations, call-graph propagation,
  diagnostics, tooling, and target-specific effect policy

## Context

Actus code must make blocking I/O, filesystem access, allocation, entropy, and
other operational boundaries visible. Today those properties are enforced only
by individual APIs and review. A compiler-recognized effect contract can make
the boundary checkable without hiding ownership, adding runtime reflection, or
turning a performance hint into a safety guarantee.

## Proposed decision

Introduce a declaration-level form:

```actus
meta effect("filesystem")
verb load_manifest(abs path: Path) -> Result[Manifest, IoError] { ... }
```

The first accepted effect vocabulary is intentionally closed and compiler
registered. Initial candidates are `blocking`, `filesystem`, `network`,
`allocation`, and `entropy`. An unknown effect name is a compile-time error;
there is no user-defined escape hatch and no wildcard effect.

Effects are properties of a verb signature. A call to an effectful verb
propagates the declared effects to its caller's checked call graph. A caller
may explicitly declare compatible effects, but a declaration must not erase,
rename, or under-report a callee effect. Native declarations require the same
explicit contract at the ABI boundary.

This ADR remains Proposed until the propagation model, recursive-call rules,
generic substitution, dynamic dispatch, target filtering, and diagnostics have
dedicated fixtures and a reviewed policy for restricted contexts such as
`meta realtime`.

## Safety invariants

- Effects are compile-time contracts with no implicit runtime representation.
- Ownership, cleanup, ABI layout, and return roles remain unchanged.
- An effect cannot authorize an operation; it only describes an already
  declared capability or operational property.
- Callers cannot suppress effects with aliases, generics, facades, or external
  declarations.
- The compiler must reject under-declared callers deterministically.
- Effect checking must not infer hidden allocation or filesystem behavior from
  names; only registered signatures and native contracts provide evidence.

## Required implementation evidence

Before acceptance, the implementation must include positive and negative
fixtures for direct calls, imported calls, nested facades, generic calls,
recursive call graphs, dynamic dispatch, external declarations, target
filtering, formatter output, LSP metadata, and stable diagnostics. The strict
suite must prove that effect metadata does not alter ownership or ABI behavior.
