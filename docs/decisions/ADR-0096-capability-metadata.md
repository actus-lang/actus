# ADR-0096: Typed target capability contracts

- Status: Accepted
- Date: 2026-10-30
- Scope: `meta capability(...)`, target availability, call propagation, and
  native boundaries

## Decision

Actus supports `meta capability("name")` as a closed, compiler-checked
requirement for a platform primitive. A capability is not a comment, a runtime
permission grant, or a replacement for an operating-system guarantee. It is a
typed declaration that must remain visible at the declaration and every call
boundary.

The initial registry is deliberately small and explicit:

- `posix.cloexec`;
- `unix.rename_atomic`;
- `windows.file_identity`;
- `windows.replace_file`.

The entries have distinct contracts: `posix.cloexec` requires close-on-exec to
be established during POSIX open; `unix.rename_atomic` requires the host's
same-filesystem atomic rename but does not imply no-replace or identity
comparison; `windows.file_identity` requires a native identity query for an
opened handle; and `windows.replace_file` requires the Windows atomic replace
primitive without claiming expected-identity matching. These are requirements
for native boundaries, not blanket permissions for every filesystem operation.

Unknown names are rejected by the parser. A declaration may contain more than
one distinct capability, but repeating the same capability is an error. The
one-name form is canonical and formatter-stable:

```actus
meta capability("posix.cloexec")
open verb open_secure(...) -> Result[File, IoError] { ... }
```

## Target contract

Capability availability is evaluated against the compiler `TargetSpec`, not
against the host running the compiler. Hosted POSIX targets provide the two
POSIX/Unix capabilities; Windows targets provide the two Windows capabilities;
freestanding targets provide none until an explicit target contract is added.
An active declaration requiring an unavailable capability is a compile-time
error before code generation. A declaration excluded by `meta target(...)` is
removed first and is not checked for another target.

The matrix is intentionally conservative. Actus never treats a missing native
primitive as available, never emulates an atomic operation with `check ->
rename`, and never turns an unsupported capability into a warning.

## Propagation and ownership

Capability requirements belong to verb signatures. A caller must declare every
capability required by a callee. This rule applies to local, imported,
qualified, external/native, generic-specialized, dynamic-role, and performance
method calls. Role implementations may not add capabilities that are absent
from the role method contract. The capability declaration does not alter
ownership roles, cleanup, serialization, or runtime state.

Capability metadata is valid on verbs, external verbs, role methods, and
performance implementations where the existing signature rules permit call
contracts. It is not file-scoped and it cannot bypass semantic, ownership,
ABI, bounded-execution, or effect validation.

## Diagnostics and compatibility

- E0021 reports an unknown capability name;
- E1131 reports a caller that omitted a callee capability;
- E1132 reports a capability unavailable for the selected target;
- E1133 reports a role implementation capability mismatch.

The registry is an initial, deliberately small set, not a claim that four
capabilities cover every platform facility. Adding a capability
requires a reviewed registry entry, target-specific implementation evidence,
parser/formatter/LSP fixtures, propagation tests, and documentation. No
arbitrary user-defined capability strings are accepted.

## Consequences

Platform dependencies become explicit at the source and at every call edge.
Native code remains responsible for proving the actual OS primitive, while the
language prevents unsupported target selection and hidden capability use. The
cost is deliberate annotation at boundaries; that cost is preferred over
silent portability failures or unsafe fallbacks.
