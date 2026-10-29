# `meta effect(...)` (proposed)

`meta effect(...)` is a proposed compiler contract for making operational
properties of a verb visible at its declaration and call sites. It is not yet
implemented and must not be used in Actus source.

The proposed form is:

```actus
meta effect("filesystem")
verb load_manifest(abs path: Path) -> Result[Manifest, IoError] { ... }
```

The initial candidate vocabulary is deliberately closed:

- `blocking`
- `filesystem`
- `network`
- `allocation`
- `entropy`

The final vocabulary and propagation rules are not accepted yet. The compiler
must reject unknown names rather than treating them as documentation-only
annotations.

## Intended contract

Effects belong to verb signatures. If a verb calls another verb with a
registered effect, that effect must remain visible in the caller's checked
contract. Imports, aliases, generic specialization, facades, external ABI
declarations, and dynamic dispatch must not hide it.

Effects do not grant permissions, insert runtime code, change ownership roles,
or imply that an operation is safe in a realtime or bounded context. A later
restricted-context attribute may consume these contracts, but that policy must
be specified separately.

The feature is ready for implementation only after its ADR is accepted and the
call-graph, recursion, dispatch, target, diagnostic, formatter, and LSP rules
are covered by strict fixtures.
