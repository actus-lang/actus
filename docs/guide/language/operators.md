# Operators

Operators are typed expressions. Their operands must satisfy the operator's
contract; Actus does not silently convert unrelated numeric types.

## Arithmetic

`+`, `-`, `*`, `/`, `%`, the wrapping forms `+%`, `-%`, `*%`, the saturating
forms `+|`, `-|`, `*|`, the bitwise operators `&`, `|`, `^`, and the compound
assignments built from them take two operands of the same integer type. The
result has that type. `Int`, `i32`, `Usize`, and `u64` are different types.
Mixing two types is the compile error `E1140`; convert one side with `as`:

```actus
erg total: u32 = (small as u32) + wide;
```

An unsuffixed integer literal takes the type of the other operand, on either
side, so `small + 1`, `1 + small`, and `small += 1` are valid when `small` is
a `u8`. The literal must fit that type: `small + 300` is rejected. A suffixed
literal such as `4u32`, a cast, and a constant expression such as `2 * 3`
keep their own type. Shift counts are not part of this rule.

## Comparisons

Comparisons produce `Bool` and require compatible operands:

```actus
if count >= limit {
    return 1;
}
```

## Logical operators

Logical operators combine `Bool` values. They use short-circuit evaluation, so
the right side of `and` or `or` may not be evaluated.

## Bitwise and shift operators

Bitwise operators and shifts work on compatible integer widths. Use explicit
fixed-width types for masks, registers, and protocol fields. Shift counts are
checked against the operand width.

## Division and remainder

Division and remainder require a valid non-zero divisor. Runtime-invalid
operations follow the compiler's checked failure or trap contract for the
selected type and target.

## Failed runtime checks

Integer overflow, division by zero, out-of-range shift counts and casts, and
indexes outside an array stop the program. A program built as an executable
reports what failed and where, then exits with status 101:

```text
error: integer overflow at src/main.act:3:17
```

Objects and freestanding builds execute the target's trap instruction
instead, because they may run without the Actus runtime (ADR-0117).

Read [types and literals](types-and-literals.md) for casts and numeric bounds,
and [the repository operator reference](../../language/operators.md) for the
complete precedence and operand table.
