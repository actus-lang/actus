# Constants and compile-time data

A `const` gives a fixed value a name that the compiler can resolve before
runtime code is generated. Constants are useful for protocol limits, masks,
array indexes, pack offsets, versions, and other values that are part of a
program's static contract.

```actus
const HEADER_BYTES: u16 = 12u16;
const PAYLOAD_OFFSET: u16 = HEADER_BYTES + 4u16;
```

The type is explicit. Use a width that matches the contract consuming the
constant, and give numeric literals a suffix when the width matters.

## Compile-time evaluation

A constant initializer may use supported typed literals, other constants, and
supported deterministic primitive expressions:

```actus
const BASE: u32 = 40u32;
const ANSWER: u32 = BASE + 2u32;

verb main() -> Int {
    return ANSWER as Int;
}
```

Dependencies may appear before or after the constant that refers to them. The
compiler resolves the dependency graph, checks the declared type and range,
and stores the resolved value for later semantic and lowering stages.

A constant is not a runtime computation. Its initializer cannot read mutable
state, call a verb, allocate storage, inspect hardware, perform I/O, or invoke
an external function. Use an `erg` binding and a verb when the value must be
computed at runtime.

## Type, range, and dependency errors

The compiler rejects:

- duplicate constant names;
- unknown names in an initializer;
- incompatible initializer types;
- values outside the declared integer range;
- cyclic constant dependencies;
- runtime calls or runtime values in a constant initializer;
- invalid constant arithmetic such as division or remainder by zero;
- shift counts that are outside the operand width.

These are semantic errors. They are reported before native code generation, so
a runtime fallback cannot turn an invalid constant into a valid layout value.

```actus
const LIMIT: u8 = 255u8;
const NEXT: u16 = LIMIT as u16 + 1u16;
```

The cast in this example is explicit. Actus does not silently widen or narrow
an integer merely because the destination constant has another width.

## Visibility and module facades

A constant is private unless it is declared `open` and exported through the
module's canonical facade:

```actus
open const WIRE_VERSION: u8 = 1u8;
```

`open` makes the declaration eligible for facade export; it does not make the
constant globally visible by itself. Follow the [module facade rules](../modules/canonical-facades.md)
when another source file must import it. Public constants need the same Actus
documentation block required for other public declarations.

Constants do not own resources and do not participate in `erg`, `abs`, `dat`,
or `ins` cleanup. A constant may be passed where its resolved value satisfies
the callee's type and role contract, but it is never moved like an owned
buffer or struct.

## Constants in pack layouts

Pack offsets may use named integer constants:

```actus
const PAYLOAD_OFFSET: u16 = 8u16;

pack Frame {
    erg storage: Array[u8, 2];
    layout little;
    fields {
        erg marker: u8 at 0;
        erg payload: u8 at PAYLOAD_OFFSET;
    }
}
```

The offset must resolve before pack validation and must fit the supported
layout-offset domain. Layout constants may use named constants and checked
integer arithmetic, but they cannot depend on calls, buffers, strings,
booleans, floating-point values, dynamic lengths, or mutable runtime data.

A named constant keeps the meaning of an offset or mask visible while still
leaving the pack layout fixed. It does not make the layout dynamic.

## Constants and fixed capacities

Constants can name fixed values used by supported compile-time positions, such
as an array index or another declaration-time integer argument. A runtime
binding cannot replace a compile-time extent:

```actus
const SLOT_COUNT: u32 = 4u32;

verb main() -> Int {
    erg values: Array[u32, 4] = Array[u32, 4]();
    values[SLOT_COUNT - 1u32] = 41u32;
    return values[3u32] as Int;
}
```

Use a const-generic parameter when the value must be part of a generic type's
identity. Use an ordinary named constant when one fixed value is sufficient.
See [generics](generics.md) for the separate const-generic contract.

## Constant versus runtime binding

| Requirement | Use |
| --- | --- |
| Fixed protocol version or bit mask | `const` |
| Compile-time pack offset | `const` |
| Fixed value reused by declarations | `const` |
| Value read from input or hardware | `erg` in a verb |
| Mutable counter or state | `erg` |
| Read-only view of an existing value | `abs` |
| Ownership transfer | `dat` at the call boundary |

Do not use a constant to hide runtime state or to bypass ownership checking.
The compiler treats a constant as static data and treats runtime bindings as
values with normal Actus lifetime and role rules.

## Related reference material

- [Types and literals](types-and-literals.md) explains integer widths and casts.
- [Packs](packs.md) explains fixed storage and field offsets.
- [Generics](generics.md) explains type and const-generic parameters.
- [Modules and facades](../modules/canonical-facades.md) explains visibility.
- [Constant reference](reference/14-constants-and-compile-time-data.md) records
  the compact compiler contract.
