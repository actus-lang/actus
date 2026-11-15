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
arguments. Resolution also produces `Unknown { spelling }` for an unresolved
user type. Its key uses the `X` tag (`unknown::<spelling>`), it is unequal to
every other identity, and it exists only long enough for semantic analysis to
emit the existing unknown-type diagnostic. Specialization, layout, and code
generation reject `Unknown` as an internal compiler error.

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

## Data flow

Package provenance starts with the normalized manifest identity and the
canonical source root. Configuration passes that pair, together with the
dependency and runtime-standard-library roots, into `ModuleResolver`. The
resolver attaches the owning package to every `ModuleIdentity`; the module
path is computed relative to that package's source root. Root packages,
dependencies, runtime `std`, and a standard-library file reached through more
than one alias therefore share an identity whenever their canonical source
root is the same. The relevant implementation locations are
`src/configuration/types.rs:18`, `src/configuration/loading.rs:110-149`,
`src/modules/resolver.rs:127-174`, and
`src/modules/aggregation/unit.rs:14-19,103-129`.

The compilation-plan declaration registry is built from those module units.
It records every declaration as `Declaration { package, module_path, name }`,
compiler declarations as `Builtin { name }`, and active generic parameters as
`GenericParameter { owner, name }`. Const-generic literal arguments are
`Const { value }`, with numeric spelling normalized before identity comparison;
named constants remain declaration identities until constant inlining resolves
them. Alias imports, open re-exports, root
imports, and `std::` paths are lookup spellings only; the registry resolves
each spelling to the same declaration identity. Resolution is cached by
`(module identity, spelling)` and returns the canonical identity in a
deterministic order.

Every `TypeName` entering semantic analysis has `resolved = Some(...)`.
Compilation-plan paths annotate the AST before semantic analysis and assert
this invariant at the semantic boundary. Standalone parsing and LSP analysis
use an explicit `resolve_standalone(program)` registry for the one-file
program. The reserved root module path is `<root>`, and a manifest-less
standalone registry uses the reserved package identity `<standalone>`; manifest
name validation rejects both because neither is an identifier. Neither name is
used by compilation-plan package resolution. There is no silent unresolved
fallback. Normalization, generic
specialization, performance-context merging, generic substitution, and
serialization-generated declarations preserve or explicitly resolve the
annotation. Generated declarations inherit the identity of their source
module.

`TypeName` retains source spelling and span for diagnostics, formatting, and
LSP display. `TypeName` equality and hashing use the identity key, the ordered
arguments, and the reference role, and never the source span. The identity key
is `Resolved(DeclarationIdentity)` when the name has been resolved and
`Unresolved(spelling)` otherwise; a resolved and an unresolved name are never
equal. Two spellings of one declaration are therefore equal after resolution,
two declarations with the same short name are not, and unresolved names
(parser, formatter, standalone tooling) keep spelling-based equality. `Hash`
is derived from the same key, so it always agrees with equality. The span-free
declaration comparison used by the A0 regression remains a separate
comparison: identical declarations with different spans remain equal, while
changes to metadata, `open` exports, or target selection remain unequal. Unit
tests cover both the identity-sensitive type comparison and the existing
span-independent declaration comparison.

The `builtin::` identity is the only builtin marker. It replaces the former
span sentinel and remains deterministic even when a user declaration has the
same short name. Diagnostics, formatter output, and LSP hover/definition
continue to use source spelling; canonical identity is used only for semantic
resolution, generic caches, specialization, layout, object dependencies, and
internal mangling.

## Semantic type checking

PR B deliberately preserves PR A's spelling-based semantic comparisons. The
semantic checker continues to compare the normalized source spelling used by
the existing diagnostics and type-mismatch rules; canonical identity is not
used to change those comparisons in this PR. Identity is already used for
declaration resolution, `GenericInstance` identity, generic substitution and
caches, specialization, layout lookup, drop and performance registries,
object dependencies, and internal (non-ABI) mangling. Semantic type
representation and identity-based comparisons are deferred to PR B3.

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
