# Declarations

Actus declarations give names to values, types, operations, and module
boundaries.

## Constants

A constant has an explicit type and a compile-time value:

```actus
const HEADER_BYTES: u16 = 12u16;
```

Constants are immutable and do not use ownership roles. See
[constants](constants.md) for evaluation and visibility rules.

## Verbs

A verb declares an operation:

```actus
verb add(abs left: u32, abs right: u32) -> u32 {
    return left + right;
}
```

The parameter role is part of the call contract. The return type is explicit.
See [verbs and contracts](verbs-and-contracts.md).

## Structs, packs, and enums

Use a `struct` for named fields with ordinary type layout, a `pack` for an
explicit bounded representation and field offsets, and an `enum` for a fixed
set of variants. Their detailed syntax is documented in their focused pages.

## External declarations

Foreign functions are declared behind an explicit unsafe boundary and must have
a documented ABI and status contract. Public Actus code should use a typed
facade over the raw external declaration.

## Documentation blocks

Public declarations use a triple-quoted Actus documentation block:

```actus
"""
Adds two non-owning integer views.

Inputs:
- left: read-only first value.
- right: read-only second value.

Returns:
- The sum as u32.
"""
verb add(abs left: u32, abs right: u32) -> u32 {
    return left + right;
}
```

Document the purpose, inputs, output, ownership, side effects, allocation,
and failure behavior when those details apply.
