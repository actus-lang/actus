# `meta repr(C)`

`meta repr(C)` is a closed, compiler-defined ABI declaration for a concrete
value type. It tells Actus that the type is intentionally usable as a C-style
native aggregate. The declaration is an assertion with checks; it is not a
request to bypass ownership, cleanup, or layout validation.

```actus
meta repr(C)
struct Point {
    x: i32,
    y: i32,
}

meta repr(C)
enum Axis {
    X,
    Y,
}
```

## Syntax and placement

The representation name is a bare compiler-defined identifier. Quoted names,
`packed`, `Rust`, arbitrary strings, and other future names are not silently
accepted. `repr(C)` may be attached only to a `struct` or `enum`; it is
rejected on verbs, external verbs, packs, roles, performances, and file
metadata. A declaration may contain at most one representation attribute.

## The checked contract

`repr(C)` is accepted only for a concrete declaration:

- generic structs and enums are rejected because their layout depends on an
  unresolved type or const parameter;
- struct fields must be ordinary value fields; `erg`, `abs`, and `ins` fields
  are ownership-bearing Actus state and cannot become C aggregate fields;
- field order is preserved and the existing neutral layout engine determines
  alignment, padding, size, and offsets;
- byte-aligned Actus integer primitives (including target-sized `Usize`),
  `Int`, `Bool`, `f32`, `f64`, and positive concrete arrays of those values are
  supported; the native target contract still determines the C consumer's
  exact scalar spelling;
- nested structs and unit enums must themselves declare `meta repr(C)`;
- strings, buffers, maps, regions, packs, opaque handles, ownership-qualified
  references, generic applications, recursive layouts, and other opaque values
  are rejected;
- enums must contain unit variants only. Payload enums require a separate
  tagged-union ABI contract and are not inferred from `repr(C)`.

There is no implicit packing. Padding remains part of the compiler-checked
layout, and `repr(C)` does not change ownership, cleanup, serialization,
effects, capabilities, determinism, realtime rules, or source limits.

## Native boundary

The attribute does not itself declare an external function or expose a private
type. It makes the type's representation explicit and gives semantic analysis
enough information to reject unsafe or ambiguous aggregate layouts before code
generation. Native declarations must still be `unsafe extern "C"` where the
foreign boundary requires it, and their ownership roles remain visible at each
parameter and return position.

`repr(packed)`, volatile access, explicit alignment overrides, tagged unions,
and target-specific layout overrides are separate contracts. They are not
aliases for `repr(C)` and require their own safety and ABI specification.

## Diagnostics and compatibility

Unsupported representation names are rejected during parsing with `E0022`.
Misplaced metadata is rejected with `E0008`. A semantically invalid C layout
is rejected with `E1134`, including the exact declaration and reason. Existing
unannotated structs and enums retain their current Actus representation and
behavior; adding `repr(C)` is opt-in and compile-time checked.
