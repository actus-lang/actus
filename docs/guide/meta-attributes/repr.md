# `meta repr(C)` and `meta repr(packed)`

`meta repr(C)` and `meta repr(packed)` are closed, compiler-defined layout
contracts for concrete value types. `repr(C)` makes a type intentionally usable
as a C-style native aggregate. `repr(packed)` removes inter-field padding and
sets aggregate alignment to one byte. Both declarations are assertions with
checks; neither bypasses ownership, cleanup, or layout validation.

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

meta repr(C, packed)
struct WireHeader {
    tag: u8,
    length: u32,
}

meta repr(C)
meta repr(packed)
struct SeparateSpelling {
    tag: u8,
    length: u32,
}
```

## Syntax and placement

Representation names are bare compiler-defined identifiers. Quoted names,
arbitrary strings, and unknown names are rejected. The supported spellings are
`repr(C)`, `repr(packed)`, and the grouped form `repr(C, packed)`. Separate
`repr(C)` and `repr(packed)` attributes are also combined on the same
declaration; repeating either component is rejected. The metadata may be
attached only to a `struct` or `enum`; it is rejected on verbs, external verbs,
packs, roles, performances, and file metadata.

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
- nested structs and unit enums must themselves declare the representation
  contract required by the containing layout; packed enums are not supported;
- strings, buffers, maps, regions, packs, opaque handles, ownership-qualified
  references, generic applications, recursive layouts, and other opaque values
  are rejected;
- enums must contain unit variants only. Payload enums require a separate
  tagged-union ABI contract and are not inferred from `repr(C)`.

For `repr(C)`, field order and the existing neutral layout engine determine
alignment, padding, size, and offsets. For `repr(packed)`, fields are laid out
in declaration order with one-byte field alignment and the aggregate alignment
is one; trailing padding is not added. A field keeps its logical type and
natural size, so an integer or floating-point field may begin at an unaligned
byte offset. Native backends lower such accesses with an explicit one-byte
alignment contract rather than assuming natural alignment.

`repr(packed)` does not change ownership, cleanup, serialization, effects,
capabilities, determinism, realtime rules, or source limits. It also does not
make a type a bit-field container: fields remain byte-sized typed values, not
arbitrary-width bit ranges.

## Native boundary

The attribute does not itself declare an external function or expose a private
type. It makes the type's representation explicit and gives semantic analysis
enough information to reject unsafe or ambiguous aggregate layouts before code
generation. Native declarations must still be `unsafe extern "C"` where the
foreign boundary requires it, and their ownership roles remain visible at each
parameter and return position.

The program-aware FFI mapper exposes this checked contract through
`c_abi_signature_in_program` and
`c_abi_external_signature_in_program`. A checked aggregate parameter is
represented as an explicit pointer plus its Actus ownership contract; it is
never silently copied into an opaque value. An owned aggregate result uses a
caller-allocated return contract, while an `abs` aggregate result is a
borrowed view. Fixed-width integers, `Bool`, `f32`, `f64`, `Usize`, and
positive concrete arrays are mapped with target-specific size, alignment, and
padding. Use the `_for_target` variants when the ABI target is not the
default 64-bit target. A type without an explicit `repr(C)` contract is
rejected before native lowering.

The mapper is descriptive and checked; it does not generate an unsafe native
call by itself. The caller must still provide the correct declaration,
ownership roles, external boundary, and target-specific native implementation.

`repr(packed)` may be combined with `repr(C)` when the packed type crosses a C
boundary. The program-aware FFI mapper reports the packed flag and exact field
offsets; the foreign declaration must use a matching packed C layout contract.
Packed enums, volatile access, explicit alignment overrides, tagged unions, and
target-specific layout overrides remain separate contracts.

## Diagnostics and compatibility

Unsupported representation names are rejected during parsing with `E0022`.
Misplaced metadata is rejected with `E0008`. A semantically invalid C layout
is rejected with `E1134`, including the exact declaration and reason. Existing
unannotated structs and enums retain their current Actus representation and
behavior; adding `repr(C)` is opt-in and compile-time checked.
