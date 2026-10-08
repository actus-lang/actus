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

`+`, `-`, `*`, `/`, and `%` operate on compatible numeric types. Integer
families preserve declared width and signedness. Do not expect automatic
promotion between `u8`, `u32`, `Int`, or signed/unsigned families.

### Comparisons

`<`, `<=`, `>`, and `>=` return `Bool`. Operands must belong to the same
numeric family: matching signed integers, matching unsigned integers, or the
same floating width. Signed and unsigned values are not implicitly mixed.

`==` and `!=` compare compatible numeric families or booleans.

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
