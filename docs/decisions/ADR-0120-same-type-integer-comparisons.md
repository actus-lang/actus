# ADR-0120: Same-Type Integer Comparisons

## Status

Accepted.

## Context

ADR-0119 made the arithmetic and bitwise operators require two operands of one
integer type. The comparison operators were left with the rule of ADR-0043
(relational operators) and ADR-0044 (equality): the operands had to be in the
same family, which for integers meant the same signedness. Width and the
integer type name were not checked, so these compiled:

```actus
erg a: u8 = 200u8;
erg b: u32 = 100000u32;
erg c: Int = 5;
erg d: i64 = 5i64;
erg x = a < b;      // u8 against u32
erg y = c == d;     // Int against i64
```

That contradicts the language rule that there is no implicit numeric
conversion, and it leaves the comparison width invisible: the back end has to
widen one operand to compare them, which is a conversion nobody wrote.

## Decision

Integer operands of `<`, `<=`, `>`, `>=`, `==` and `!=` have the same integer
type. Two operands have the same type when their type names are equal, exactly
as in ADR-0119: `Int`, `i32`, `Usize` and `u64` are four different types.

When the types differ, the compiler reports `E1140` at the right operand:

```text
operands of `<` must have the same integer type, found `u8` and `u32`; convert one side explicitly with `as`
```

This also covers operands that differ only in signedness, such as `i8` and
`u8`, which previously reported the operator type mismatch `E1025`. They now
report `E1140`, because the operands are integers and differ in type. The author
converts one side with `as`: `(a as u32) < b`.

### Literals

An unsuffixed integer literal on either side, including a negated or
parenthesized one, takes the type of the other operand and must fit it, with
the same notion of literal as ADR-0119. `5 < a` and `a < 5` are valid for
`a: u8`; `300 < a` and `-1 < a` are the literal-range error `E1068`, reported
at the literal. A suffixed literal, a cast and a constant expression keep their
own type.

The rule is shared with ADR-0119: one check, `validate_same_type_operands`,
serves both families, selected by `BinaryOp::requires_same_operand_type` and
`BinaryOp::is_comparison`.

### Unchanged

- Floating-point comparisons: both operands have the same width.
- `Bool` equality and equality of enums whose variants carry no payload.
- Any other pair of operand types keeps its operator type mismatch `E1025`.
  This includes an integer literal compared with a floating-point value.

### Code generation

Both back ends already retyped a literal operand of a comparison to the type
of the other operand, and lowering already retypes a literal on the left from
the source expression for every operator except shifts (ADR-0119). `100 < a`
with `a: u8 = 200u8` therefore runs as an unsigned 8-bit comparison on LLVM and
Cranelift.

## Consequences

- This extends ADR-0119 to comparisons. It supersedes the signedness-only rule
  for integers in ADR-0043 and ADR-0044, which stay in force for floating-point
  operands, `Bool` and enums. ADR-0119 says that relational and equality
  operators keep their rule; that sentence is historical and is replaced by
  this decision.
- A program that compared two different integer types no longer compiles. The
  fix is an explicit `as` on one operand.
- `E1140` now covers comparisons as well as arithmetic and bitwise operators.
  A mixed-signedness integer comparison reports `E1140` instead of `E1025`.
- The standard library and the examples did not mix integer types in a
  comparison. Two places did: the checksum guard that the compiler generates
  for `serialize` decoders compared an `Int` result with `0i64` and now
  compares it with the literal `0`, and one fixture in `tests/arrays_cli.rs`
  compared a `u12` pack field with a `u16` literal and now converts the field
  with `as u16`.
- A constant operand is reported where it is used. Lowering inlines constant
  values before `actus check` and `actus build` validate the program, and the
  inlined value used to keep the spans of the declaration, so `a == LIMIT`
  pointed at the value of `LIMIT`, in another file when the constant was
  imported. The inlined value now takes the span of the name it replaces. This
  applies to every diagnostic on such an operand, including `E1140` from
  ADR-0119.
- `docs/language/diagnostics.md`, the operator reference and the user guide
  describe the rule.
