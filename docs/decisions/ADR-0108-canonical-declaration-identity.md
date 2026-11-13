# ADR-0108: Canonical Declaration Identity

## Status

Proposed

## Context

R1 is a specialization failure where one enum reached through two `open`
re-export spellings is instantiated as two different types, even though both
spellings resolve to the same declaration.

ADR-0068 defines generic type identity but does not define the owning package,
the canonical module path, builtin declarations, re-export resolution, or a
compilation-session resolution cache.

## Decision

A declaration identity is the owning package, the declaration's module path
relative to that package's source root, the declaration name, and ordered type
arguments.

The owning package is the package whose source root contains the declaring
file; the module path is the file's module path relative to that source root.
Import paths, aliases and re-exports never contribute to identity. Therefore,
two paths such as `path` and `std::path` share one identity when they resolve to
the same source declaration.

The identity namespace of a package is its manifest name with `-` replaced by
`_`. The result must be an identifier; names that are not identifiers after
mapping, names that collide after mapping within one compilation, and the
reserved names `std` and `builtin` are rejected with a diagnostic. `std` is
reserved for the compiler-owned standard-library package.

Generated declarations, including serialization verbs and synthetic
declarations, receive the identity of the module containing their source
declaration.

Identity namespaces:

- `builtin::` for compiler-provided declarations that have no source file;
- `std` for the compiler-owned standard-library package, which has source
  files and follows the same package + module path rule;
- the normalized package name for package declarations.

For example, a result type is represented as
`builtin::Result[builtin::Int, app::Error]`.

Resolution results are cached for the duration of one compilation session.
The cache is keyed by canonical source/declaration identity and preserves
deterministic resolution order.

The `builtin::` namespace replaces the `definition.span.start == 0` builtin
sentinel currently used by `src/semantic/analyzer/verb.rs:70`.

The E1110 namespace rule is unchanged. Relaxing or changing that rule is a
separate language decision.

## Consequences

- Generic specialization, generic caches, layout lookup, object dependencies,
  and internal mangling use one canonical declaration identity.
- Source spelling and source span remain available for diagnostics.
- Re-export spelling does not affect specialization or layout.
- LSP hover/definition output is unchanged for users.
- Internal specialized names and non-public identity keys may change.
- `extern "C"` names and public native ABI symbols remain unchanged.
- Resolution no longer repeats the same canonical lookup within one session.

## Compatibility

Compatible — bug fix. Programs that already build retain their observable
behavior. Programs that were incorrectly specialized through two re-export
spellings now use one declaration identity. Manifest-name validation is new;
no manifest in the repository is affected.

Internal mangling may change, but `extern "C"` and public native ABI symbols
remain stable. The E1110 rule is unchanged.

## Alternatives

### Short declaration name

Rejected because it makes codegen correctness depend on the E1110 namespace
rule, which is a separate language decision. Full declaration identity is
correct independently of that rule.

### Identity only in `GenericInstance`

Rejected because semantic resolution, generic caches, specialization, layout,
object dependencies, mangling, and LSP lookup must share the same identity.


## References

- [ADR-0068: Canonical Generic Type Identity](ADR-0068-canonical-generic-type-identity.md)
- [ADR-0106: Source Layout and Module Facades](ADR-0106-source-layout-and-module-facades.md)
- [Phase 53 alpha-release readiness](../roadmap/phase-53-alpha-release-readiness.md)
- Implementation location: `src/configuration/loading.rs:252`
