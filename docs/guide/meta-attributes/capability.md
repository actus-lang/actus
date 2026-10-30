# `meta capability(...)`

`meta capability("name", ...)` declares one or more typed requirements for
platform primitives. The registry is closed and target-aware. It is not a
documentation string and it does not grant a verb permission to perform an
operation.

```actus
meta capability("posix.cloexec")
open verb open_secure(...) -> Result[File, IoError] { ... }
```

Distinct requirements may be grouped in one declaration:

```actus
meta capability("windows.file_identity", "windows.replace_file")
open verb publish(...) -> Result[Int, IoError] { ... }
```

## Supported names

| Name | What it authorizes the declaration to require | Supported target contract |
| --- | --- | --- |
| `posix.cloexec` | Request close-on-exec behavior when a POSIX file descriptor is opened. The runtime must set the flag as part of the open operation; a later flag update is not an equivalent implementation. | hosted POSIX targets |
| `unix.rename_atomic` | Require the host's atomic same-filesystem rename primitive for publication. It does not promise no-replace semantics, identity comparison, or cross-filesystem support. | hosted non-Windows Unix targets |
| `windows.file_identity` | Require the Windows native file-identity query needed to compare an opened file's identity, rather than comparing names or timestamps. | Windows targets |
| `windows.replace_file` | Require the Windows native replacement primitive for an atomic replacement publication. It does not by itself prove expected-identity matching or recovery after an interrupted operation. | Windows targets |

These names describe required host primitives, not broad permissions. For
example, `windows.replace_file` does not mean that every rename is safe, and
`unix.rename_atomic` does not mean that a destination identity was verified.
The implementation must still validate paths, ownership, failure ordering, and
the relevant publication contract separately.

Unknown names are compile-time errors. The same capability cannot be repeated
on one declaration. The one-name form remains valid, while grouped names are
preferred when one boundary requires multiple primitives. The formatter
preserves capability order.

## Registry scope and extension policy

The four entries are the initial registry required by the current native file
and publication boundaries; they are not intended to cover every operating
system facility. Other useful capabilities may later include an explicitly
scoped Linux no-replace rename primitive, directory-handle synchronization,
Windows handle inheritance control, OS entropy, or platform-specific socket
operations. They must not be added as speculative strings. Each new entry
requires a precise semantic meaning, an implementation that calls the native
primitive directly, target-specific positive and negative fixtures, and a
reviewed compatibility update to this table and ADR-0096. A capability whose
semantics cannot be proved on a target remains unsupported there.

## Propagation

If a callee declares a capability, its caller must declare the same capability.
The requirement crosses imports, qualified names, external/native declarations,
generic specialization, dynamic role dispatch, and performance implementations.
Role implementations cannot silently add a capability that the role method did
not declare.

```actus
meta capability("posix.cloexec")
verb primitive() -> Int { return 0; }

meta capability("posix.cloexec")
verb caller() -> Int {
    primitive();
    return 0;
}
```

Omitting the second declaration is E1131. The capability name is checked
against the selected `TargetSpec`; an unavailable active capability is E1132.
Declarations excluded by `meta target(...)` are filtered before capability
availability is checked.

## Safety boundary

Capability metadata does not change ownership, effects, bounded proofs,
determinism, realtime restrictions, ABI layout, or cleanup. It cannot emulate a
missing operating-system primitive and it cannot turn a non-atomic fallback
into an atomic operation. Native/runtime code must still implement and test the
declared primitive for each supported target.
