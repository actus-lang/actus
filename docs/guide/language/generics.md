# Generic types and verbs

Generics make one declaration work with a set of explicitly checked type or
const arguments.

```actus
verb first_value[T](abs values: Array[T, 2]) -> T {
    return values[0];
}
```

The element type and fixed extent are part of the instantiated type. The
compiler validates generic arguments, role bounds, fixed layouts, and nested
instances before native lowering.

## Const generics

A const generic represents a compile-time numeric extent or layout value:

```actus
verb zeroed[N](erg values: Array[u8, N]) -> Void {
    for index in 0u32..N {
        values[index] = 0u8;
    }
}
```

Use the supported const-generic domain and explicit bounded storage. A runtime
value cannot replace a declaration-time extent.

## Generic ownership

Generic parameters must preserve the ownership role required by the operation.
A generic declaration cannot hide a move, borrow, or mutable loan. Nested calls
are checked after concrete specialization and before native emission.

Generic enums and results retain their discriminant and payload layout for each
valid instance. Unsupported or unsized element types are rejected at semantic
analysis.
