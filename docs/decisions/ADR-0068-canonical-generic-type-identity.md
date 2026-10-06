# ADR-0068: Canonical Generic Type Identity

- Status: Accepted for the Phase 35 compiler implementation
- Date: 2026-10-06

## Context

Generic nominal types can reach the compiler through different source spans,
public facade paths, and call sites. Those routes may render the same type name
while still producing separate internal values. Native emission exposed the
problem when a generic result crossed a nested facade: semantic checking
accepted the value, but native lowering could not reconcile the resulting
return representation.

The compiler also has two different naming needs:

1. semantic and cache layers need a stable structural identity for deciding
   whether two generic instantiations are the same type;
2. native definitions and ABI planning need a name that preserves information
   which affects representation, including ownership roles in a type
   application such as `Option[abs PathComponent]`.

Using either name for both purposes makes one layer lose information required
by the other. Source location and module traversal details must not create a
second semantic type, but they may still be retained as diagnostic or cache
context metadata.

## Decision

Actus uses one structured `TypeIdentity` value for the structural identity of
nominal type applications. It contains:

- the nominal declaration name;
- the ordered identities of all type arguments;
- const arguments represented by their type-level value names until Actus has
  a separate typed const identity representation.

`TypeIdentity` is immutable, comparable, hashable, and serializable to a
deterministic key for compiler-internal maps. Argument order is significant:
`Pair[Int, Bool]` and `Pair[Bool, Int]` are different identities.

The following data is excluded from structural identity:

- source spans;
- ownership roles on references (`erg`, `abs`, `dat`, and `ins`);
- parent or nested facade paths;
- call-site locations;
- display strings and diagnostic wording.

Ownership roles are excluded because they describe access and lifetime at a
use site. They are still validated by semantic analysis and remain relevant to
the ABI and layout contract. Native specialized definitions therefore retain
the ABI-aware canonical name derived from the original generic instance. For
example, the structural key `Option[PathComponent]` and the ABI-aware native
name `Option[abs PathComponent]` are intentionally different representations
of the same semantic generic declaration at different compiler boundaries.

## Pipeline contract

Every pipeline boundary follows this rule:

| Boundary | Identity contract |
| --- | --- |
| Semantic analysis | Record one `TypeIdentity` per nominal generic application. |
| Facade export propagation | Reuse the structural identity; facade traversal never forks it. |
| Generic cache | Index structural instances by toolchain, identity, caller context, and source call span. The latter two are discovery context, not type identity. |
| Specialization | Select instances by `TypeIdentity`; preserve the ABI-aware canonical name on generated native definitions. |
| Layout registry | Key generic layouts by structural identity and compute representation from the substituted type and target configuration. |
| Native dependency planning | Compare reachable generic instances by structural identity and caller context. |
| Native emission | Use ABI-aware canonical names and full native signatures for definitions, declarations, and symbols. |

The native symbol binding table is scoped by module namespace and source
declaration. A unique-source compatibility fallback is allowed only when one
source declaration has one unambiguous symbol. Distinct declarations with the
same source name must remain distinct.

## Cache ownership and invalidation

Generic instance caches are compiler-owned and immutable after a compilation
plan is materialized. A cache entry is invalidated when any of these change:

- the compiler toolchain identity;
- the structural generic identity;
- the generic caller context;
- the source call span used to distinguish independent discovered instances;
- the target or ABI configuration used to compute native layout.

The cache may deduplicate equivalent structural instances inside one
toolchain, but it must not merge different ordered arguments, callers, target
contracts, or ABI definitions. Cache keys are internal implementation data and
must not be persisted as application state or serialized into Actus runtime
formats.

## Compatibility and ownership

This decision does not change Actus source syntax, ownership rules, generic
argument validation, `Result` or `Option` layout, serialized formats, or public
facade visibility. It does not add implicit conversions between generic
instantiations. A real ABI mismatch remains an error and cannot be hidden by
structural identity matching.

## Evidence

- `TypeIdentity` structural equality tests exclude source spans and roles.
- Generic cache tests prove structural deduplication within one toolchain.
- Layout tests key substituted layouts by `TypeIdentity`.
- The test-only specialization trace proves that structural
  `Result[Item]` and ABI-aware `Result[abs Item]` remain distinct at the
  appropriate boundaries.
- Direct and nested facade native regression tests pass with 25 application
  tests and 0 failures.

## Consequences

The compiler can reuse one semantic generic identity across facade paths while
preserving the representation information required by native lowering. Source
locations remain available for diagnostics and cache discovery without
creating phantom types. Future const-generic or ABI identity extensions must
extend the structured identity contract and its tests rather than introduce a
second string-based identity scheme.

## Migration note

Compiler-internal generic cache keys now use `TypeIdentity` instead of a
display or source-location-derived string. Native specialized definitions keep
their ABI-aware canonical names, and external symbol bindings are scoped by
module namespace. This changes internal cache and symbol-planning behavior
only; Actus source syntax, public facade imports, ownership annotations,
serialized formats, and native ABI contracts remain source-compatible.
