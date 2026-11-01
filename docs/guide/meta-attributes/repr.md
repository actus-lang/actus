# `meta repr(C)`, `meta repr(packed)`, `meta repr(align(...))`, and `meta repr(transparent)`

These are closed, compiler-defined layout contracts for concrete value types.
`repr(C)` makes a type intentionally usable as a C-style native aggregate.
`repr(packed)` removes inter-field padding and sets aggregate alignment to one
byte. `repr(align(N))` raises the aggregate's minimum alignment to the
compiler-checked power-of-two byte value `N`. `repr(transparent)` makes a one-field wrapper use exactly the underlying
field's layout and C ABI type while preserving a distinct Actus source type.
All three declarations are assertions with checks; none bypasses ownership,
cleanup, or layout validation.

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

meta repr(C, align(8u32))
struct AlignedHeader {
    tag: u8,
}

meta repr(align(16u32))
struct SeparatelyAligned {
    tag: u8,
}

meta repr(transparent)
struct UserId {
    value: u64,
}
```

## Syntax and placement

Representation names are bare compiler-defined identifiers. Quoted names,
arbitrary strings, and unknown names are rejected. The supported spellings are
`repr(C)`, `repr(packed)`, `repr(transparent)`, and the grouped form
`repr(C, packed)`. Separate `repr(C)` and `repr(packed)` attributes are also
combined on the same declaration; repeating any component is rejected.
`repr(transparent)` is intentionally not composable with `C`, `packed`, or
`align`. `align(N)` requires an explicit positive power-of-two `u32` literal;
it may be written in the same `repr(...)` group or as a separate component,
but repeating the component is rejected.
The metadata may be attached only to a `struct` or `enum`; it is rejected on
verbs, external verbs, packs, roles, performances, and file metadata.

## The checked contract

`repr(C)`, `repr(packed)`, and `repr(align(N))` are accepted only for a
concrete struct declaration:

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

For `repr(align(N))` specifically:

- `N` must be a positive power of two and must use the `u32` literal suffix;
- the requested value is a minimum: natural field alignment is never lowered;
- the final aggregate size is rounded up to the resulting alignment;
- it cannot be combined with `repr(packed)` or `repr(transparent)`;
- a type crossing the native FFI boundary must also declare `repr(C)`;
- explicit alignment does not grant native access or make unsupported fields ABI-safe.

For payload enums, `repr(C)` selects the explicit tagged-union contract:

- the discriminant is a compiler-defined `u32` at offset zero;
- the payload union begins after the discriminant at the next valid payload
  alignment boundary;
- each variant receives a stable zero-based discriminant in declaration order;
- the payload storage is large enough for the largest variant and uses the
  greatest payload alignment; the final type size is rounded to the aggregate
  alignment;
- unit variants have no payload, while tuple payload fields are named by their
  zero-based position and struct payload fields retain their declared names;
- payload fields must satisfy the same fixed-layout, value-role, nested
  representation, recursion, and generic checks as C structs;
- the native mapping is a C-compatible struct containing a `uint32_t` tag and
  a union-shaped payload. It is not emitted as a C `enum`, because C enums
  cannot carry payload bytes.

`repr(transparent)` is a separate wrapper contract:

- the declaration must be a concrete struct with exactly one field;
- the field must be an ordinary value field, not `erg`, `abs`, or `ins`;
- the field type must satisfy the same checked fixed-layout rules as a nested
  representation contract, including explicit representation on nested
  aggregates;
- `repr(transparent)` cannot be combined with `repr(C)` or `repr(packed)`;
- the wrapper has the field's size and alignment, and field access remains at
  offset zero; nested transparent wrappers are resolved recursively;
- transparent enums, generic wrappers, opaque values, recursive layouts, and
  ownership-qualified references are rejected.

For `repr(C)`, field order and the existing neutral layout engine determine
alignment, padding, size, and offsets. For `repr(align(N))`, the layout engine
uses the greater of the natural aggregate alignment and `N`, then rounds the
size to that alignment. For `repr(packed)`, fields are laid out
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
`c_abi_external_signature_in_program`. A checked C aggregate parameter is
represented as an explicit pointer plus its Actus ownership contract; it is
never silently copied into an opaque value. An owned aggregate result uses a
caller-allocated return contract, while an `abs` aggregate result is a
borrowed view. Transparent wrappers are different: the mapper recursively
resolves the single field and exposes the underlying C ABI scalar, array, or
checked aggregate type directly. This prevents a nominal wrapper from
introducing an accidental ABI layer. Fixed-width integers, `Bool`, `f32`,
`f64`, `Usize`, and positive concrete arrays are mapped with target-specific
size, alignment, and padding. Use the `_for_target` variants when the ABI
target is not the default 64-bit target. A type without an explicit
representation contract is rejected before native lowering.

The mapper is descriptive and checked; it does not generate an unsafe native
call by itself. The caller must still provide the correct declaration,
ownership roles, external boundary, and target-specific native implementation.

`repr(packed)` may be combined with `repr(C)` when the packed type crosses a C
boundary. The program-aware FFI mapper reports the packed flag and exact field
offsets; the foreign declaration must use a matching packed C layout contract.
Payload enums are mapped as checked tagged unions and are passed using the same
aggregate pointer contract as C structs. Transparent wrappers are mapped directly to the underlying ABI type and do not
produce an aggregate pointer layer. Explicit alignment is reported in the
checked aggregate layout and requires `repr(C)` for FFI mapping. Packed enums,
volatile access, tagged unions, and target-specific layout overrides remain
separate contracts.

## Diagnostics and compatibility

Unsupported representation names are rejected during parsing with `E0022`.
Misplaced metadata is rejected with `E0008`. A semantically invalid C layout
is rejected with `E1134`, including the exact declaration and reason. Existing
unannotated structs and enums retain their current Actus representation and
behavior; adding `repr(C)` is opt-in and compile-time checked.
