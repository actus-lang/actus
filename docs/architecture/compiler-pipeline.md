# Actus Compiler Pipeline

Actus uses a one-directional compilation pipeline:

```text
source -> lexer -> parser -> AST -> semantic analyzer -> native codegen
```

- The lexer produces tokens and source spans.
- The parser builds syntax-only AST nodes and does not perform ownership
  checking.
- The AST is the frontend representation shared by later phases.
- The semantic analyzer resolves names, types, ownership, borrow records,
  receiver roles, and cleanup plans.
- The native backend emits code only after semantic analysis succeeds.

Diagnostics are data owned by the frontend and are rendered separately by the
CLI. Backend-specific types do not cross into lexer, parser, or AST modules.
Dependencies must move forward through the pipeline; reverse dependencies are
not permitted.

## Hierarchical module units

Module resolution produces one deterministic compilation unit for a canonical
parent facade and its explicitly opened child facades. Each directory module
has a facade whose filename matches the directory. Child implementation files
remain in the child facade's internal scope; external resolution receives only
the export table assembled by the parent facade chain. Direct child imports
are rejected before semantic analysis.

The resolver, parser aggregation, semantic visibility checks, native object
planning, formatter, LSP, and test runner consume this same module-unit
identity. Filesystem order is not an API contract, and a child directory must
not create a second object owner when its parent module is compiled.

## Native dependency closure

Native object emission begins from the requested entry verb or the public
facade roots and computes a deterministic transitive closure of Actus verbs.
Reachable private helpers are emitted in their owning object; unreachable
private verbs are omitted. Generic verbs are specialized before declaration and
lowering, so each concrete instance has a deterministic symbol identity that
is preserved across root-to-module linker bindings.

## Canonical generic identity

The semantic, facade, cache, layout, and native dependency stages share the
structured `TypeIdentity` contract defined by ADR-0068. Its structural key is
the nominal declaration name plus ordered type arguments; source spans, facade
paths, call-site locations, and display strings remain metadata. This prevents
one generic application from becoming multiple compiler types merely because
it crossed a facade or was discovered at another source location.

Native definitions retain the ABI-aware canonical name when ownership roles or
other representation details affect the native contract. Structural identity
selects the specialization; the ABI-aware name and signature validate the
emitted representation. Object and executable emission therefore consume the
same resolved identity and cannot silently merge incompatible layouts.

The closure walk includes nested blocks, conditional expressions, indexed
places, method/performance calls, aggregate return dependencies, and external
bridge declarations. Built-in constructors and runtime intrinsics are not
treated as Actus verb bodies. An unresolved native dependency fails closed at
the originating call span with both the caller and missing helper name; it
must never be silently dropped and discovered later as an opaque linker error.

This contract is shared by object builds, executable builds, and the test
runner. Formatter and source-limit checks remain independent of native
reachability. Any change to call collection, generic specialization, module
facades, or symbol binding must include semantic, native, and multi-object
regression evidence.

Compile-time constants use the same facade dependency graph. Before native
identifier lowering, each imported object receives the public constant
declarations reachable through its transitive canonical-facade imports. Private
constants remain excluded, repeated declarations are deduplicated by identity,
and constants do not become runtime or ABI symbols. This keeps semantic,
object, executable, test-runner, and LSP visibility contracts aligned.
