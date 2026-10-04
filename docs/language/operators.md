# Actus Operator Reference

Version: 0.1

Status: Implemented Alpha language surface

This reference describes operators implemented by the compiler. Operator
validation happens before native code generation; Actus does not insert
implicit numeric conversions.

## Precedence and result types

Operators bind from tighter to looser as follows:

| Precedence | Operators | Result |
| --- | --- | --- |
| 1 | unary `!`, `~`, `-` | operand-dependent |
| 2 | `*`, `/`, `%` | numeric/integer |
| 3 | `+`, `-` | numeric |
| 4 | `<<`, `>>` | left integer type |
| 5 | `&` | left integer type |
| 6 | `^` | left integer type |
| 7 | `\|` | left integer type |
| 8 | `<`, `<=`, `>`, `>=` | `Bool` |
| 9 | `==`, `!=` | `Bool` |
| 10 | `&&` | `Bool` |
| 11 | `\|\|` | `Bool` |

Binary operators are left-associative. Parentheses explicitly override this
table.

## Operand contracts

- Relational operators require matching signed integer, unsigned integer, or
  equal-width floating-point families. Signed and unsigned integers are not
  mixed implicitly.
- `==` and `!=` compare compatible numeric families or `Bool` values and
  return `Bool`.
- `%` requires integer operands. A zero divisor is a deterministic runtime
  trap unless the semantic analyzer can prove the divisor is zero, in which
  case it reports `E1092` before code generation.
- `&&` and `||` require `Bool` operands and short-circuit the right operand.
  `!` requires and returns `Bool`.
- `&`, `|`, `^`, and `~` require integer operands and preserve the validated
  integer width and signedness.
- Shifts require an integer left operand and an unsigned integer count. A
  count outside the destination width traps at runtime; a statically known
  invalid count reports `E1093`.

## Checked casts and indexing

`expr as Type` is an explicit checked primitive integer cast. Constant values
outside the target range are rejected during semantic analysis. Dynamic casts
perform a native range check and trap on overflow or underflow. A cast never
changes ownership or allocates.

`Array[T, N]` and `Buffer` support dynamic indexing with `Int`, `Usize`, and
unsigned primitive integer types. Array capacity and Buffer length are checked
before the native load or store; an invalid index traps deterministically.
Array slots may be passed as exclusive `ins` loans without copying the array.

Eligible scalar values can be reused explicitly with `copy(value: abs value)`.
The operation requires the `abs` role and is limited to compiler-approved
integer and boolean scalars. It preserves the scalar type and does not consume
the caller. Buffers, strings, resources, cleanup-bearing aggregates, and
unsupported user-defined types are rejected.

Pack fields are read and written through native shift/mask lowering. Source
code uses field access rather than manually reproducing the packed layout.

## Accepted examples

```actus
verb classify(erg sample: u8, erg limit: u8) -> Bool {
    return sample < limit && sample != (0 as u8);
}

verb mask(erg input: u8) -> u8 {
    erg count: u8 = 1;
    erg mask: u8 = 0x0F;
    return (input & mask) << count;
}
```

## Rejected examples

```actus
// Signed and unsigned values are not implicitly converted.
verb invalid(erg left: i8, erg right: u8) -> Bool {
    return left < right;
}

// The semantic analyzer rejects a statically known zero divisor.
verb invalid_remainder() -> Int {
    return 7 % 0;
}
```
