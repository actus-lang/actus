# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 18. Roles and performance implementations

A role is a compile-time callable contract. Every method has an explicit
receiver parameter and the receiver role is part of the contract:

```act
struct Cursor {
    position: Int,
}

open role Reader {
    verb read(ins self: Cursor, ins buffer: Buffer) -> Result[Int, IoError];
}
```

A concrete type implements the contract with `perform`:

```act
perform Reader for Cursor {
    open verb read(ins self: Cursor, ins buffer: Buffer) -> Result[Int, IoError] {
        return Result[Int, IoError].Ok(0);
    }
}
```

The implementation must provide every role method and preserve names, receiver
roles, parameter roles, parameter types, return types, and failure types.
Missing methods, unknown roles, duplicate methods, and mismatched receivers are
semantic errors.

Static performance dispatch is the normal path. The compiler resolves a
concrete implementation and records the reachable performance for native
lowering. Dynamic dispatch is an explicit runtime contract, written with a
`dynamic` role value, and must not be introduced just to avoid a generic type
error.

A `dat` receiver consumes the caller's owner. An `ins` receiver creates an
exclusive call-scoped loan. An `abs` receiver reads without ownership. A
`Drop` performance uses `ins self` so cleanup cannot run while the value is
frozen or already loaned.
