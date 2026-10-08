# Operators

Operators are typed expressions. Their operands must satisfy the operator's
contract; Actus does not silently convert unrelated numeric types.

## Arithmetic

`+`, `-`, `*`, and the checked arithmetic forms operate on compatible integer
or supported numeric types. The result keeps the declared type. Use explicit
casts when values have different widths or signedness.

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

Read [types and literals](types-and-literals.md) for casts and numeric bounds,
and [the repository operator reference](../../language/operators.md) for the
complete precedence and operand table.
