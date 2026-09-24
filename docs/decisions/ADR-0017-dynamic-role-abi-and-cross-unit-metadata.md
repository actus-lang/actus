# ADR-0017: Dynamic Role ABI and Cross-Unit Metadata

- Status: Accepted
- Date: 2026-09-25
- Scope: Alpha dynamic role objects and future Actus unit linking

## Context

Actus now supports explicit `dynamic` role objects inside one native build.
Cross-unit use requires a deterministic contract for the fat pointer, vtable
entries, calling convention, and `.actmeta` compatibility checks. Without
that contract, a consumer could link a vtable with a different method order,
pointer width, role signature, or compiler toolchain.

The internal Actus ABI remains unstable during Alpha. This decision defines
the Alpha compatibility identity and metadata needed before cross-unit dynamic
linking is enabled; it does not make the internal ABI a stable public ABI.

## Decision

### Dynamic object representation

`abs dynamic Role` is a borrowed, non-owning fat pointer with two target-sized
machine words:

| Word | Field | Meaning |
| --- | --- | --- |
| 0 | `data_ptr` | Address of the concrete object instance |
| 1 | `vtable_ptr` | Address of the role vtable for that concrete type |

The pair uses the selected `TargetSpec` pointer width, alignment, endianness,
and native calling convention. Passing a dynamic role through an Actus unit
boundary expands it to two ABI words in the same order. The receiver passed to
a vtable method is `data_ptr`; `vtable_ptr` is dispatch metadata and is never
an ownership handle.

Only `abs dynamic Role` is valid. A dynamic role object cannot be an `erg` or
`dat` parameter, cannot own the concrete object, and cannot outlive the owner
from which its `data_ptr` was borrowed.

### Vtable symbols and slots

Each concrete `perform Role for Type` reachable by a dynamic call generates a
deterministically named vtable data object:

```text
actus_vtable_<encoded-role>_<encoded-native-target>
```

The encoding keeps ASCII letters and digits and represents every other byte
as an underscore followed by two lowercase hexadecimal digits. Vtable entries
are target-sized function pointers. Method slots are ordered by the canonical
method symbol order recorded in metadata. Each slot records the role method
name, zero-based slot, receiver role, parameter roles and types, return type,
and native calling convention.

The vtable function pointer at the selected slot is loaded and invoked with
`data_ptr` as the first argument. The call is indirect; it never changes the
ownership role of the dynamic object.

### `.actmeta` dynamic interface records

The unit interface metadata remains TOML and must contain the existing
compatibility identity:

```text
format_version
unit
compiler_version
toolchain_hash
target_triple
target_spec_hash
profile
```

Before a unit exports or imports a dynamic role, its metadata must also carry
deterministic dynamic interface records equivalent to:

```toml
[[dynamic_roles]]
role = "Writer"
layout = "data_ptr,vtable_ptr"
pointer_width = 8
alignment = 8
vtable_symbol = "actus_vtable_Writer_struct_3"

[[dynamic_roles.methods]]
name = "write"
slot = 0
receiver = "abs"
signature = "(abs self: File) -> Int"
calling_convention = "target-native"
```

The serialized representation must preserve declaration-independent ordering:
dynamic roles sort by role name, and methods sort by canonical method symbol.
The metadata records the concrete native target key, role contract identity,
vtable symbol, slot, and complete semantic signature. It never embeds an
implementation body.

### Compatibility and rejection

An Actus unit may participate in cross-unit dynamic linking only when all of
these fields match:

- metadata format version;
- compiler version and toolchain hash;
- target triple and target specification hash;
- profile and native calling convention;
- dynamic role name and method signatures;
- fat-pointer layout and vtable slot map.

The compiler or Arca must reject an incompatible `.actmeta` before binary
linking. When source is available, the unit is rebuilt with the current
toolchain. When source is unavailable, the build fails with a deterministic
diagnostic identifying the mismatched field. A stale vtable must never be
silently reused.

This contract is not the stable C ABI. C FFI cannot consume an Actus dynamic
role object unless a separate explicit C adapter contract is defined.

## Implementation Gate

Cross-unit dynamic linking remains disabled until the compiler and Arca
implement the metadata records, import/export validation, vtable symbol
visibility, and incompatible-unit rejection described here. The current
single-build native implementation is valid for Alpha internal testing.

## Consequences

- Dynamic dispatch has an explicit and auditable ABI rather than an implicit
  runtime convention.
- Target identity and toolchain identity protect vtable layout correctness.
- Static `perform` dispatch remains free of fat pointers and vtable metadata.
- Cross-unit support gains a clear implementation boundary without freezing
  the entire internal Actus ABI prematurely.
