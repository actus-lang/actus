# Types, literals, and numeric safety

Actus types describe the width, layout, ownership-relevant shape, and failure
behavior of values.

## Primitive types

Common primitive types include `Bool`, `Int`, fixed-width unsigned integers
such as `u8`, `u16`, `u32`, and `u64`, and fixed-width signed integer types.
Use fixed-width integers when a width is part of a protocol, file, register, or
ABI contract.

Integer literals may include an explicit suffix:

```actus
erg byte: u8 = 0u8;
erg count: u32 = 12u32;
erg status: Int = 0;
```

Use a suffix when the intended width matters. Unsuffixed literals may receive a
known integer type in supported checked contexts when the value fits.

## Explicit casts

Conversions use `as`:

```actus
erg count: u32 = 12u32;
return count as Int;
```

Actus does not silently promote or convert incompatible numeric types. A cast
is checked for the target type's range. Static overflow is rejected during
semantic analysis; runtime-only overflow follows the native checked path.

## Boolean values

Conditions require `Bool`. Integer values are not implicitly treated as true or
false:

```actus
if ready {
    return 0;
}
```

Use comparisons or boolean operators to produce a condition.

## Option and Result

`Option[T]` represents a value that may be present. `Result[T, E]` represents a
successful value or a typed error. Handle both with `case` patterns rather
than assuming a value exists.

```actus
case result {
    Result[u32, DecodeError].Ok(value) => {
        return value as Int;
    },
    Result[u32, DecodeError].Err(error) => {
        return 1;
    },
}
```

The `?` operator can propagate a compatible `Result` error from a verb whose
return type permits that error.

## User-defined types

Structs group named fields, packs describe explicit storage layouts, enums list
variants, and generic types describe reusable bounded shapes. Follow the links
in the language index for their complete syntax.
