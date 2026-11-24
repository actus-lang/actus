# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 12. Operators

Precedence, from tightest to loosest:

| Level | Operators |
|---:|---|
| 1 | unary `!`, `~`, `-` |
| 2 | `*`, `/`, `%` |
| 3 | `+`, `-` |
| 4 | `<<`, `>>` |
| 5 | `&` |
| 6 | `^` |
| 7 | `|` |
| 8 | `<`, `<=`, `>`, `>=` |
| 9 | `==`, `!=` |
| 10 | `&&` |
| 11 | `||` |

Binary operators are left-associative. Parentheses override precedence.

### Numeric operators

`+`, `-`, `*`, `/`, and `%` (and the wrapping `+%`, `-%`, `*%`, the
saturating `+|`, `-|`, `*|`, and `&`, `|`, `^`) take two operands of the same
integer type and produce that type. Integer families preserve declared width
and signedness. There is no automatic promotion between `u8`, `u32`, `Int`,
or signed/unsigned families: operands of different types are rejected with
`E1140`, and one side is converted with `as`. Compound assignments such as
`+=` follow the same rule (ADR-0119).

An unsuffixed integer literal operand, on either side, takes the type of the
other operand and must fit it. A suffixed literal, a cast, and a constant
expression such as `2 * 3` keep their own type.

### Comparisons

`<`, `<=`, `>`, and `>=` return `Bool`. Integer operands must have the same
integer type, so `u8` and `u32`, `i32` and `Int`, or `i8` and `u8` are rejected
with `E1140`; convert one side with `as`. Floating-point operands must have
the same width. An unsuffixed integer literal on either side takes the type of
the other operand and must fit it (ADR-0120).

`==` and `!=` follow the same rule for integers and also compare `Bool`
values and enums whose variants carry no payload.

### Logical operators

`&&` and `||` require booleans and short-circuit the right side. `!` requires
and returns `Bool`. Do not use bitwise operators as boolean operators unless
the operands are explicitly integer masks.

### Bitwise and shift operators

`&`, `|`, `^`, and `~` require integer operands and preserve the validated
width/signedness. Shifts require an integer left operand and an unsigned count.
An invalid count traps at runtime or is rejected statically when known.

### Division and remainder

Division and remainder by zero are deterministic failures. A statically known
zero divisor is rejected during semantic analysis. A dynamic zero divisor is
handled by the native runtime failure path.
