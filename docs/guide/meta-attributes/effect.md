# `meta effect(...)`

`meta effect(...)` is a compiler contract for making operational
properties of a verb visible at its declaration and call sites. The compiler
parses the closed vocabulary, preserves it in verb signatures, and rejects
callers that omit a callee's declared effect.

The canonical form groups all effects declared by one verb:

```actus
meta effect("filesystem", "blocking")
open verb load_manifest(abs path: Path) -> Result[Manifest, IoError] { ... }
```

The one-effect form remains valid for compatibility:

```actus
meta effect("filesystem")
```

The compiler stores each name as a separate effect in the verb signature.
Grouping is syntax only: duplicate detection, effect propagation, and caller
requirements remain unchanged. The formatter emits adjacent effect metadata in
the grouped canonical form.

The supported vocabulary is deliberately closed:

- `blocking`
- `filesystem`
- `network`
- `allocation`
- `entropy`
- `volatile`

The vocabulary and propagation rules are compiler contracts. The compiler must
reject unknown names rather than treating them as documentation-only
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

## Role and dynamic-dispatch contracts

Role methods may declare effects:

```actus
role Writer {
    meta effect("filesystem")
    verb write(abs self: File) -> Int;
}

perform Writer for File {
    meta effect("filesystem")
    verb write(abs self: File) -> Int { return 0; }
}
```

The role method and every implementation must carry exactly the same effect
set. An implementation may not omit a role effect or add an effect that the
role interface does not expose, because either case would make a dynamic or
generic call graph unsound. Dynamic and generic role-bound calls therefore use
the role method's declared effects when checking their caller.

External ABI declarations use the same rule as ordinary verbs:

```actus
meta effect("network")
extern "C" verb host_call() -> Int;
```

The caller must explicitly declare `network`; the metadata does not authorize
the native operation or insert a runtime guard.

`volatile` marks an explicit hardware-observable memory access. It is not a
permission to use arbitrary addresses: the operation must still use a checked
native memory contract. The effect propagates to callers and is rejected from
deterministic and realtime contexts.

The contract is compile-time only. It does not authorize an operation, alter
ownership, or by itself make a verb safe for a realtime or bounded context.
