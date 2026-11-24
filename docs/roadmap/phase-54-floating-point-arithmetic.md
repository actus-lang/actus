# Phase 54: Floating-Point Arithmetic

## Status: Planned

`f32` and `f64` are declared primitive types, but today they are only stored
and compared. A program can bind a float literal and compare two floats of the
same width; every other float operation is rejected or missing:

- `+`, `-`, `*` and `/` on floats are rejected with `E1025`; the message names
  `integer` as the accepted operand type.
- `as` converts only between integer types, so `x as Int` with `x: f64` is
  `E1090`.
- `print` and `format` have no float argument.
- The standard library has no math on floats.

Control loops, sensor filtering and geometry need all of these, so floats are
not usable for real programs until this phase lands.

## Goals

1. **Arithmetic.** `+`, `-`, `*`, `/` and unary `-` on `f32` and `f64`, on the
   LLVM and Cranelift back ends. Both operands have the same float type, as
   ADR-0119 requires for integers; an unsuffixed float literal takes the type
   of the other operand. The wrapping, saturating and bitwise operators and
   `%` stay integer-only unless an ADR decides otherwise.
2. **IEEE 754 semantics.** Float arithmetic follows IEEE 754 and does not trap:
   overflow gives an infinity and invalid operations give `NaN`. An ADR fixes
   what `==`, `!=` and the relational operators mean for `NaN` and signed zero,
   and whether constant folding may evaluate float expressions.
3. **Conversions.** `as` between integer and float types and between `f32` and
   `f64`. A float-to-integer conversion traps with an ADR-0117 report when the
   value is `NaN`, infinite or out of range, like an integer conversion that
   does not fit. Integer-to-float and `f64`-to-`f32` conversions round to
   nearest.
4. **Output.** `print(float: ...)`, `eprint(float: ...)` and a float argument
   for `format`, with a documented shortest round-trip representation.
5. **Standard library.** `abs`, `min`, `max`, `sqrt`, `floor`, `ceil`, `round`
   and the basic trigonometric functions for `f32` and `f64`, with their
   behaviour on `NaN` and infinities documented.

## Known defects to fix first

- A negative float literal such as `-1.5f64` passes `actus check` and then fails
  LLVM module verification, because negation is lowered with an integer
  overflow intrinsic (`llvm.usub.with.overflow.f64`). This is tracked as an
  0.1.3 bug and is fixed before this phase starts.

## Exit criteria

- Each goal has semantic tests and CLI tests that build and run on both back
  ends, and every new test has been seen failing.
- The operator reference, the type reference and `docs/language/diagnostics.md`
  describe the float rules; the type reference no longer says floats work "only
  where backend and contract support it" without saying what that is.
- An ADR records the arithmetic, `NaN` and conversion decisions.
