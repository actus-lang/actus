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

There is no general implicit numeric conversion. If a `u32` operation needs a
`u32` operand, write `1u32` or use an explicit cast. Do not solve type errors by
changing a protocol field to `Int` unless the ABI and width contract really
requires it.
