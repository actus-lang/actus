# ADR-0072: Native ABI for Const-Generic Aggregate Parameters

## Status

Accepted.

## Context

Const-generic aggregate values already had native layout support when they
were created locally or returned through a caller-owned slot. A downstream
scale fixture exposed a missing composition: a specialized aggregate such as
`Fabric[256]`, whose storage contains a bounded array of packed elements, was
passed through an `ins` or `abs` parameter of a generic verb.

Strict semantic checking accepted the program, but native emission reported:

```text
native backend cannot lower parameter `fabric` of type `Fabric`
```

The concrete aggregate layout instance was not retained after generic verb
specialization. Typed local declarations also retained symbolic annotations
such as `Fabric[N]` instead of receiving the caller's concrete substitution.

## Decision

After generic verb specialization, native preparation collects every concrete
generic struct and enum type application present in specialized declarations.
Those instances are added to the layout and ABI registry using their complete
canonical identity, including const arguments such as `Fabric[256]`.

Typed owner declarations in specialized verb bodies apply the same type and
const substitutions as parameters, return types, and expressions. This keeps
the semantic type, aggregate layout, ownership role, and native parameter ABI
consistent.

Aggregate parameters continue to use the existing native representation:
bounded aggregates are lowered through their established indirect or direct
ABI classification, and `ins`/`abs` ownership remains compiler-checked. No
pointer or dynamic allocation is introduced by this change.

## Invariants

- Const arguments remain positive compile-time `Usize` values.
- Specialized aggregate layout names retain their complete type identity.
- Pack-backed arrays preserve element size, alignment, and contiguous stride.
- `ins`, `abs`, `erg`, and `dat` roles retain their existing ownership rules.
- Generic, direct concrete, facade, and nested generic calls use the same ABI.
- Invalid or unbounded aggregate applications remain rejected by semantic
  validation.

## Verification

Native regressions cover a pack-backed aggregate passed through `ins` at
`Fabric[64]`, `Fabric[256]`, and `Fabric[1024]`, the same aggregate through an
`abs` parameter, and the same composition through a nested facade. The full
compiler quality suite, source-limit checks, and downstream Twin-e strict
scale benchmark are required before closing the associated Phase 36 gate.
