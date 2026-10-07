# ADR-0075: Canonical Native Symbols for Generic External ABI Declarations

- Status: Accepted
- Date: 2026-10-07
- Scope: Generic external declarations, native symbol binding, and runtime ABI

## Context

Actus generic specialization creates an internal name such as
`actus_region_open__u32`. That name is useful for compiler lookup, but it is
not a valid replacement for the stable symbol supplied by a runtime or C
library. Emitting the specialization name at the external boundary makes a
valid generic declaration fail at link time because the runtime exports
`actus_region_open`.

## Decision

External declarations carry optional canonical native symbol metadata. Generic
specialization preserves the original external symbol in this metadata while
retaining the specialized Actus declaration name for compiler-local call
resolution. Native declaration and object symbol collection use the canonical
metadata when present.

Declarations representing imported Actus wrappers keep the existing namespace
binding. Only a real external boundary receives canonical native symbol
metadata; this prevents module wrapper symbols from being treated as raw C
symbols.

## Invariants

- Generic specialization names never leak into a real external ABI symbol.
- The canonical symbol is unchanged by the concrete type arguments.
- Imported Actus wrappers continue to use their namespace-qualified symbols.
- ABI signature validation remains responsible for rejecting incompatible
  declarations that claim the same native symbol.
- No runtime allocation, pointer, or application-specific workaround is added
  by this compiler rule.

## Evidence

- `generic_external_abi_uses_the_canonical_native_symbol` proves that
  `bridge[u32]` imports `bridge` rather than `bridge__u32`.
- The `std::region` object emission regression passes after generic
  `size_of[T]` specialization and canonical bridge naming are both enabled.
