# Generic types and verbs

Generics let one declaration describe several explicitly checked concrete
instances. Actus supports generic structs, enums, verbs, and selected
standard-library types. A generic declaration is not an untyped template: each
use is checked for arity, argument kinds, ownership, layout, and role bounds.

## Type parameters

A type parameter is written in square brackets after the declaration name:

```actus
struct Box[T] {
    erg item: T,
    label: Int,
}

verb identity[T](erg item: T) -> T {
    return item;
}

verb main() -> Int {
    erg boxed: Box[Int] = Box[Int] {
        item: 41,
        label: 1,
    };
    return identity[Int](item: boxed.item) + boxed.label;
}
```

`Box[Int]` is a concrete type. `Box[Option[Int]]` is another concrete type,
with its own field layout and ownership behavior. The compiler does not treat
`Box[Int]` and `Box[u32]` as interchangeable merely because both arguments are
integer-like.

The number and kind of arguments must match the declaration. Missing, extra, or
misordered arguments are rejected before native lowering. Generic parameters
are scoped to the declaration where they are introduced; a parameter from one
verb or type cannot be used by another declaration unless it declares its own
parameter.

Generic enums and results keep their typed payloads:

```actus
verb make_result[T](erg value: T) -> Result[T, Int] {
    return Result[T, Int].Ok(value);
}
```

Pattern matching still handles the concrete enum variants and preserves the
normal ownership rules for the extracted payload.

## Ownership in generic code

A generic parameter does not erase ownership. The role on a parameter remains
part of the instantiated call contract:

```actus
verb consume[T](dat value: T) -> T {
    return value;
}
```

When `T` is specialized with an owning type, the call transfers that value. A
generic function cannot hide a move, borrow, or mutable loan behind the type
parameter. Nested generic calls are checked after their concrete arguments are
known and before native code is emitted.

The same applies to generic fields. If `T` is a `Buffer`, a `Box[T]` contains
an owned resource and follows normal cleanup. If `T` is a scalar, cleanup has
no resource to release.

## Const-generic parameters

A const parameter represents a compile-time bounded number rather than a type
or a mutable runtime binding. The current supported domain is `Usize`:

```actus
struct Cell {
    erg charge: u8,
}

struct Fabric[N: Usize] {
    erg cells: Array[Cell, N],
}

verb main() -> Int {
    erg fabric: Fabric[2] = Fabric[2] {
        cells: Array[Cell, 2](),
    };
    fabric.cells[0].charge = 41u8;
    return fabric.cells[0].charge as Int + 1;
}
```

The concrete value is part of the type identity and layout. `Fabric[2]` and
`Fabric[256]` are different specialized types. The compiler resolves the
positive representable argument before semantic layout and native lowering.

A const parameter can also be used inside a generic verb as a compile-time
expression value:

```actus
verb capacity[N: Usize]() -> u32 {
    if 0u32 < (N as u32) {
        return N as u32;
    }
    return 0u32;
}

verb main() -> Int {
    return capacity[4]() as Int;
}
```

The value may participate in supported casts, comparisons, arithmetic,
conditions, indexing, and nested generic calls after specialization. It does
not become mutable runtime storage.

The current const-generic contract accepts a positive compile-time `Usize`
literal or an already-declared const parameter. It rejects zero, negative or
overflowing values, runtime expressions, type applications, and
role-qualified arguments in the const position. Do not assume arbitrary
constant arithmetic or every compile-time context is supported just because
`N` is available inside a generic body.

## Role bounds and static dispatch

A type parameter can require one or more declared roles:

```actus
struct Input {
    state: Int,
}

role Reader {
    verb read(abs self: Input, ins buffer: Buffer) -> Int;
}

struct Adapter[Source: Reader] {
    erg source: Source,
    erg buffer: Buffer,
}

verb refill[Source: Reader](ins adapter: Adapter[Source]) -> Int {
    return adapter.source.read(buffer: ins adapter.buffer);
}
```

The concrete type must provide a matching `perform` implementation for every
required role. Multiple bounds use `+`:

```actus
role Serializable {
    verb encode(abs self: Input) -> Int;
}

verb encode[T: Reader + Serializable](abs value: T) -> Int {
    return 0;
}
```

Role bounds are simple declared role names. An applied role such as
`Reader[Int]` is not a valid bound. Dispatch is primarily static: the compiler
selects the concrete implementation during specialization and creates a
deterministic native call boundary. Generic bounds are not Rust-style trait
inference, specialization, associated types, or arbitrary metaprogramming.

## Fixed-layout generic values

A generic type may contain arrays, packs, and other fixed-size aggregates. A
specialization is valid only when all concrete fields have a known native
layout. APIs that require fixed storage, such as a region element or an array
slot, reject unsized or unsupported specializations during semantic analysis.

A generic declaration may defer some validation until specialization. This is
useful for a generic region or aggregate, but it does not weaken the final
layout check. Every concrete instantiation must still satisfy its target type
contract.

## Common generic errors

| Error | Meaning | Correction |
| --- | --- | --- |
| Generic arity mismatch | The number of supplied arguments is wrong. | Supply exactly the declared arguments. |
| Unknown type parameter | A parameter is used outside its declaration. | Declare it on the current type or verb. |
| Const constraint mismatch | The const argument is not a valid positive `Usize`. | Use a supported compile-time argument. |
| Role constraint mismatch | The concrete type has no required `perform`. | Add the matching implementation or use another type. |
| Type mismatch | Two concrete applications are distinct types. | Use the exact specialization required by the callee. |
| Invalid fixed layout | A specialization contains an unsupported or unsized value. | Use a fixed-size element or change the API boundary. |

## What generics do not provide

Actus generics do not provide implicit runtime reflection, unbounded
metaprogramming, dynamic allocation, or automatic conversion between
specializations. Keep capacity, ownership, role bounds, and ABI behavior
visible in the generic declaration.

## Related reference material

- [Types reference](reference/6-types.md) contains the complete generic and
  const-generic contract.
- [Structs](structs.md) explains generic aggregate fields and cleanup.
- [Ownership roles](../ownership/roles.md) explains roles preserved through
  specialization.
- [Roles and `perform`](reference/18-roles-and-performance-implementations.md)
  explains the implementation boundary.
- [Arena placement](arenas.md) explains fixed-layout generic values in bounded
  storage.
