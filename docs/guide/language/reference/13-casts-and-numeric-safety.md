# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 13. Casts and numeric safety

The explicit cast form is:

```act
erg byte: u8 = value as u8;
```

`as` is a checked primitive integer cast. It does not allocate and does not
change ownership. Constant values outside the target range are rejected
during semantic analysis. Dynamic values use a native range check and fail
deterministically on overflow or underflow.

Prefix operators bind tighter than `as`: `-x as i64` is `(-x) as i64`, so the
negation happens in the type of `x` and traps if it overflows. A negated
literal is a single signed constant, which makes `-128 as i8` valid and
`-1 as u8` the compile error `E1091`. Use `-(x as i64)` to cast first and
negate afterwards.

There is no general implicit numeric conversion. If a `u32` operation needs a
`u32` operand, write `1u32` or use an explicit cast. This includes arithmetic
and bitwise operators: `a + b` with `a: u8` and `b: u32` is rejected (`E1140`)
and is written `(a as u32) + b`. Only an unsuffixed literal adapts to the other
operand. Do not solve type errors by
changing a protocol field to `Int` unless the ABI and width contract really
requires it.
