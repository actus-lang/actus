# ADR-0096: Typed target capability contracts

- Status: Accepted
- Date: 2026-10-30
- Scope: `meta capability(...)`, target availability, call propagation, and
  native boundaries

## Decision

Actus supports `meta capability("name", ...)` as a closed, compiler-checked
requirement for one or more platform primitives. A capability is not a comment, a runtime
permission grant, or a replacement for an operating-system guarantee. It is a
typed declaration that must remain visible at the declaration and every call
boundary.

The initial registry is deliberately small and explicit:

- `posix.cloexec`;
- `unix.rename_atomic`;
- `windows.file_identity`;
- `windows.replace_file`.

This registry is intentionally extensible. The following capabilities are
planned candidates for future review; they are not supported by this ADR until
their native implementation, target contract, diagnostics, and tests exist:

| Candidate | Intended contract | Candidate targets |
| --- | --- | --- |
| `posix.openat` | Require descriptor-relative path resolution rooted at an opened directory descriptor. | hosted POSIX targets |
| `posix.no_follow` | Require kernel-enforced no-follow path resolution for the operation. | hosted POSIX targets with the required open primitive |
| `posix.create_exclusive` | Require kernel-atomic create-new semantics that fail when the destination exists. | hosted POSIX targets |
| `posix.rename_noreplace` | Require a native no-replace rename primitive; it must not be emulated by `check -> rename`. | hosted Unix targets that expose it |
| `posix.fsync` | Require durable file-content synchronization before publication is acknowledged. | hosted POSIX targets |
| `posix.directory_fsync` | Require durable directory-entry synchronization after publication or removal. | hosted POSIX targets that support it |
| `posix.file_identity` | Require native device/inode identity retrieval for an opened object. | hosted POSIX targets |
| `unix.link_atomic` | Require an atomic hard-link operation for a publication protocol. | hosted non-Windows Unix targets |
| `unix.symlink` | Require native symbolic-link creation with the platform's documented path semantics. | hosted non-Windows Unix targets |
| `windows.create_new` | Require Windows create-new disposition with no overwrite of an existing destination. | Windows targets |
| `windows.no_follow` | Require Windows reparse-point-safe path opening for the operation. | Windows targets with the required native flags |
| `windows.handle_inheritance` | Require explicit inheritance control at handle creation, not a later best-effort update. | Windows targets |
| `windows.flush_file_buffers` | Require native file-buffer flushing before the operation reports durable progress. | Windows targets |
| `windows.directory_flush` | Require a supported native directory-entry durability primitive. | Windows targets that expose it |
| `windows.file_lock` | Require an OS-enforced advisory or mandatory file-lock contract with documented sharing semantics. | Windows targets |
| `posix.file_lock` | Require an OS-enforced file-lock contract with documented advisory semantics. | hosted POSIX targets |
| `runtime.entropy` | Require cryptographically suitable OS entropy for security-sensitive generation. | hosted targets with an approved entropy source |
| `runtime.monotonic_clock` | Require a monotonic clock source that cannot move backward during one process lifetime. | hosted targets |

These candidates are a planning inventory, not an authorization list. A name
must not be used in source until it is added to the closed compiler registry.
Each addition must define the exact guarantee, unsupported-target behavior,
failure semantics, ownership and blocking behavior, and whether the primitive
is atomic, durable, identity-aware, or merely advisory. Platform aliases are
not interchangeable: a capability is added only when the target-specific
primitive proves the declared contract.

The entries have distinct contracts: `posix.cloexec` requires close-on-exec to
be established during POSIX open; `unix.rename_atomic` requires the host's
same-filesystem atomic rename but does not imply no-replace or identity
comparison; `windows.file_identity` requires a native identity query for an
opened handle; and `windows.replace_file` requires the Windows atomic replace
primitive without claiming expected-identity matching. These are requirements
for native boundaries, not blanket permissions for every filesystem operation.

Unknown names are rejected by the parser. A declaration may contain more than
one distinct capability, but repeating the same capability is an error. The
grouped form is canonical when multiple requirements are needed; the one-name
form remains valid and formatter-stable:

```actus
meta capability("windows.file_identity", "windows.replace_file")
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
