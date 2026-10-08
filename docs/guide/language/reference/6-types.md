# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 6. Types

### 6.1 Built-in and primitive types

The compiler knows these central types:

| Type | Meaning | Typical use |
|---|---|---|
| `Int` | implementation-level integer used by the current hosted/runtime ABI | exit codes, runtime status values, simple counters |
| `Bool` | boolean truth value | conditions, flags, comparisons |
| `String` | text value used by the string ABI | text output and text APIs |
| `Buffer` | owned, bounded byte storage with live length and capacity | packets, files, byte-oriented I/O |
| `Array[T, N]` | fixed-capacity contiguous storage of `N` values of `T` | embedded tables, bounded state, fixed frames |
| `Map` | registered built-in type name; current native support is limited and must be checked before use | do not assume a full map API |
| `Void` | no-value return type | procedures and side-effect-only verbs |
| `Usize` | target-sized unsigned index family | array and buffer indexing |
| `u1` through `u128` | unsigned fixed-width integer families | bit fields, protocol fields, registers |
| `i1` through `i128` | signed fixed-width integer families | signed bounded arithmetic |
| `f32` | 32-bit floating-point family | only where backend and contract support it |
| `f64` | 64-bit floating-point family | only where backend and contract support it |

The parser accepts integer widths from 1 through 128 in the primitive type
registry. Native support and useful arithmetic semantics still depend on the
backend and the individual operation. Do not claim that every width has the
same ABI or instruction quality on every target.

### 6.2 User-defined types

- `struct Name { ... }` models a product/aggregate type.
- `enum Name { ... }` models a tagged union/sum type.
- `pack Name { ... }` models an explicitly laid out storage representation.
- `role Name { ... }` models a callable contract.
- `Array[T, N]` and generic declarations compose existing types.

### 6.3 Generic types

Generic parameters are written in square brackets:

```act
struct Box[T] {
    erg item: T,
}

verb identity[T](erg item: T) -> T {
    return item;
}
```

Role-bounded generic declarations use a bound:

```act
open struct BufferedReader[Source: Reader] {
    erg source: Source,
    erg buffer: Buffer,
}
```

Generic dispatch is primarily static. A concrete instantiation must have
deterministic layout and native symbol generation. Do not assume Rust-like
trait inference, specialization, associated types, or arbitrary generic
metaprogramming.

#### 6.3.1 Const generic parameters

Actus supports a production const-generic slice. A const parameter is distinct
from a type parameter and represents a compile-time bounded numeric value, not
mutable runtime storage:

```act
struct Record {
    erg marker: u8,
}

struct Fabric[N: Usize] {
    erg records: Array[Record, N],
}

verb make_fabric() -> Fabric[8] {
    return Fabric[8] {
        records: Array[Record, 8](),
    };
}
```

The currently supported and tested domain is `Usize`, with positive,
representable integer literal arguments used as `Array[T, N]` capacities. The
compiler preserves the const argument in generic identity, layout identity,
specialized native symbols, and deterministic cache/build decisions.

The following are required compiler behaviors:

- `N: Usize` is parsed as a const parameter, not as an invalid primitive type
  or an ordinary type bound;
- the argument is resolved before semantic layout and native lowering;
- runtime expressions are rejected in const-generic positions;
- negative, zero, overflowing, or otherwise unrepresentable capacities are
  rejected before code generation;
- the const value is not silently allocated as mutable runtime state;
- missing arguments, wrong domains, duplicate parameters, and unresolved
  const references receive deterministic diagnostics.

Const expressions and additional compile-time positions beyond the bounded
`Array` capacity slice are not silently implied by this feature. However, once
a const parameter is declared on a generic verb, its value may be used as a
compile-time expression value inside that verb:

```act
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

The compiler specializes `N` to the concrete literal before native lowering.
This works through casts, binary expressions, conditions, indexing, and
nested expression arguments. Explicit generic verb calls use the same square
bracket syntax as generic type applications. A const argument must still be
a positive compile-time `Usize` literal or an already-declared const
parameter; runtime expressions, zero, nested type applications, and
reference-role-qualified arguments are rejected.

Do not assume that arbitrary constant arithmetic is accepted as a generic
argument, or that a const parameter is a mutable runtime binding. Use a
named `const` for reusable compile-time expressions outside a generic
parameter's specialization contract.

### 6.4 `Option` and `Result`

The compiler supplies the canonical generic enum shapes:

```act
enum Option[T] {
    Some(T),
    None,
}

enum Result[T, E] {
    Ok(T),
    Err(E),
}
```

Use `Option[T]` when a value may be absent without using null. Use
`Result[T, E]` when an operation can succeed or fail with a typed error.

```act
verb lookup(abs input: Buffer) -> Option[Int] {
    return Option[Int].Some(7);
}

verb read_value() -> Result[Int, IoError] {
    return Result[Int, IoError].Ok(7);
}
```

Variant construction uses `Type[Arguments].Variant(...)` for a known generic
type or `Result.Ok(...)`/`Result.Err(...)` where inference has enough context.
Pattern matching extracts payloads. `?` propagates a compatible `Err` from a
fallible expression.

When native lowering extracts a regular enum payload from another enum, it
materializes an owned payload allocation before the binding is used. This keeps
`dat` transfers and cleanup on valid allocation boundaries; inline struct
payloads continue to use their containing storage directly. Compiler changes
must preserve this distinction and cover both accepted execution and cleanup
regressions.
