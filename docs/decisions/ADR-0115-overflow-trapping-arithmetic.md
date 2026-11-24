# ADR-0115: Overflow-Trapping Arithmetic

## Status

Accepted. Implemented in two steps: the operators first, then checked
arithmetic.

## Context

`+`, `-`, `*` and unary `-` on integers wrap in two's complement in every
profile. A `u8` computation of `250 + 10` silently yields `4`. Division and
remainder by zero, `MIN / -1`, shift counts, casts and indexing already trap,
so arithmetic overflow is the one integer failure that passes unnoticed: a
size computed as `count * item_size` can wrap to a small value and the
program continues with a wrong number. Rust traps only in debug builds and
wraps in release; C leaves signed overflow undefined.

Some code needs wrap-around on purpose: hash functions, CRCs, checksums and
sequence counters. Control code needs results that stop at the type's bound,
such as a motor command or a sensor sum. Code that reads sizes from outside
the program needs to detect overflow and handle it without stopping.

## Decision

### Trapping operators

`+`, `-`, `*`, unary `-` and the compound assignments `+=`, `-=` and `*=`
trap when the mathematical result does not fit the operand type, in every
profile, on every backend. Signed and unsigned types both trap: `0u8 - 1u8`
traps. The trap is the same deterministic trap as division by zero.

Floating-point arithmetic is unchanged.

### Wrapping operators

`+%`, `-%`, `*%`, unary `-%` and `+%=`, `-%=`, `*%=` compute the result modulo
2^N for an N-bit integer, which is today's behavior of `+`, `-`, `*` and `-`.
They require integer operands.

### Saturating operators

`+|`, `-|`, `*|` and `+|=`, `-|=`, `*|=` clamp the result to the type's
minimum or maximum. For `u8`, `250 +| 10` is `255` and `5 -| 10` is `0`. They
require integer operands.

Precedence of each new operator equals that of its trapping form: `*%` and
`*|` bind like `*`, the others like `+` and `-`. Mixing forms in one
expression is allowed; parentheses keep the reading clear.

### Checked arithmetic

The builtin verbs `checked_add`, `checked_sub` and `checked_mul` take two
integers of one type and return `Option[T]`: `Option.Some(result)` when the
result fits and `Option.None` when it overflows. They are declared in the
`builtin` identity namespace (ADR-0108), so they need no import and a program
cannot declare a verb that shadows them. This step follows the operators.

### Constant overflow

A trapping `+`, `-`, `*` or `/` whose operands are integer literals, possibly
nested and negated, and whose result does not fit its type is rejected
before code generation with the new diagnostic `E1138`:
`erg x: u8 = 200u8 + 100u8;` does not build. Wrapping and saturating
operators never report it. An operation on a named constant traps at run
time like any other operation.

### Lowering

The backend instruction set gains trapping and saturating variants of add,
subtract, multiply and negate; the existing operations keep wrapping
semantics and remain what compiler-generated arithmetic (loop counters,
offsets) uses. LLVM lowers trapping operations to the
`llvm.{s,u}{add,sub,mul}.with.overflow` intrinsics with a branch to
`llvm.trap`, and saturating add and subtract to `llvm.{s,u}{add,sub}.sat`.
Cranelift lowers them to its `*_overflow` instructions with a trap. The
optimizer removes an overflow check only when it proves the operation cannot
overflow, for example a loop counter bounded by its exit condition.

### Benchmark gate

Each benchmark kernel exists twice. `<kernel>.act` uses wrapping arithmetic,
the arithmetic of its C twin, so the comparison with C keeps measuring what
Actus hands LLVM. `<kernel>_checked.act` uses trapping arithmetic; it is held
by its own baseline, and the gate reports its cost over the wrapping kernel.
At introduction the checks cost nothing on the collatz and sieve kernels,
25% on xorshift and 53% on recursive Fibonacci, where the check on the sum
stops LLVM from turning the recursion into a loop, and 140% on the struct
loop, which stays below C.

## Consequences

- Code that relied on silent wrap-around traps until it uses the wrapping
  operators. This is a breaking change. The standard library, examples and
  test programs needed no migration: none relied on wrap-around.
- Every arithmetic operation on integers gains a check. The benchmark gate
  (ADR-0114) measures the cost; its baselines move in the same change.
- Six binary operators, unary `-%`, six compound assignments, `E1138`, and
  the
  `checked_*` builtins become language surface; the operator reference,
  the alpha guarantees and the diagnostics table document them.
