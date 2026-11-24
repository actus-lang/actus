# ADR-0119: Same-Type Integer Operands

## Status

Accepted.

## Context

The language has no implicit numeric conversion. A binding, an argument, a
return value and a comparison between different integer types are rejected,
and the user guide says so. The binary arithmetic and bitwise operators were
the exception: they accepted any two integer types and gave the result the
type of the left operand.

```actus
erg a: u8 = 250u8;
erg b: u32 = 100000u32;
erg c = a + b;      // accepted, and computed as u8
```

This hides two kinds of mistake. The result is narrowed or widened to the left
type without a visible cast, so `u32 + i32`, `i8 + u8` and `u8 + Int` change
meaning or trap depending on which side is written first. And a reader cannot
tell from the expression which width the arithmetic runs in.

## Decision

Both operands of an arithmetic or bitwise binary operator have the same
integer type. The operators are:

- `+`, `-`, `*`, `/`, `%`
- the wrapping forms `+%`, `-%`, `*%`
- the saturating forms `+|`, `-|`, `*|`
- `&`, `|`, `^`
- every compound assignment built from them: `+=`, `-=`, `*=`, `/=`, `%=`,
  `+%=`, `-%=`, `*%=`, `+|=`, `-|=`, `*|=`, `&=`, `|=`, `^=`

Two operands have the same type when their type names are equal. `Int`, `i32`,
`Usize` and `u64` are four different types. The result has the common type.

When the operand types differ, the compiler reports `E1140`, at the right
operand, naming both types and the fix:

```text
operands of `+` must have the same integer type, found `u8` and `u32`; convert one side explicitly with `as`
```

The author converts one side with `as`, which is checked and traps when the
value does not fit: `(a as u32) + b`.

### Literals

An unsuffixed integer literal operand takes the type of the other operand.
That covers a literal on either side, a negated literal and a parenthesized
literal: `x + 1`, `1 + x`, `x - -1`, `x & 0xFF`, `x += 1`.

The literal must fit the type it takes. `x + 300` with `x: u8` is the existing
literal-range error `E1068`, reported at the literal. A literal on the left
gives the whole expression the type on the right, so `erg y: u8 = 1 + x`
is accepted when `x: u8`.

These are not literals for this rule, and keep their own type:

- a suffixed literal such as `4u32`;
- a cast, so `x + (4 as u32)` with `x: u8` is rejected;
- a constant expression such as `2 * 3`, which has type `Int`; write the value
  or convert it with `as`.

### Operators that are not in this rule

- Shifts `<<` and `>>` keep their rule: the left operand is any integer, the
  count is an unsigned integer of any width, and the result has the type of
  the left operand. The count is a separate quantity, not a second value of
  the shifted type, so `a << n` with `a: u8` and `n: u32` stays valid and a
  signed count is still rejected with `E1025`.
- Relational and equality operators keep their rule from ADR-0043 and
  ADR-0044. This decision does not change them.
- Floating-point operands: this decision does not change how they are
  handled; it concerns integer types only.

### Code generation

The back ends already retyped a literal operand to the type of the other
operand, but they identified a literal by its value, not by the source. A
binding that holds a constant looked like a literal, so `1 + a` with
`a: u8 = 255u8` retyped `a` to `Int` and computed 256 instead of trapping.
Lowering now retypes a literal on the left from the source expression, so the
generated arithmetic has the type that semantic analysis reports.

## Consequences

- A program that mixed integer types in one operator no longer compiles. The
  fix is an explicit `as` on one operand; the value does not change when the
  conversion is exact.
- Code that relied on the result taking the left type now has to say which
  type it wants, which makes the width of the arithmetic visible.
- In this repository `examples/hardware_register.act`,
  `examples/operator_surface.act` and two fixtures in `tests/pack_cli.rs`
  summed fields of different widths into an `Int`; they convert each field
  with `as Int`. The standard library and the other examples did not mix
  types.
- Compound assignment to a field or an array element that mixed types was
  already rejected by the place's own type check (`E1033`, `E1087`). It now
  reports `E1140`, like a compound assignment to a binding.
- `E1140` is documented in `docs/language/diagnostics.md`. The operator
  reference and the user guide describe the rule.
