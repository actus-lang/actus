# Expressions and statements

Actus distinguishes expressions that produce values from statements that only
perform an action or change control flow.

## Statement conditionals

A statement `if` performs a guarded action:

```actus
if index >= count {
    break;
}
```

Its branches do not produce a value. The condition must be `Bool`.

## Conditional expressions

An expression `if` produces one value:

```actus
erg selected: u32 = if ready {
    primary
} else {
    fallback
};
```

The value branches must have compatible types, unless one branch diverges with
`return`, `break`, or `continue`.

## Loops

Use a bounded `for` when the iteration range is known and a `loop` when early
termination or carried state is the clearer representation:

```actus
for index in 0u32..count {
    inspect(value: abs values[index]);
}
```

`break` leaves the nearest loop and `continue` begins its next iteration.
Ownership cleanup is performed at the relevant scope boundary.

## Assignment

Assignment changes an owned mutable place:

```actus
erg count: u32 = 0u32;
count = 1u32;
count += 1u32;
```

The left side must be writable and its type must match the assigned value.
Indexed and field places retain their bounds and ownership checks.

## Operators

Arithmetic, comparisons, logical operators, bitwise operators, shifts,
division, and remainder are typed operations. Read the [operator reference](../../language/operators.md)
for precedence and operand contracts.

## Case

`case` selects among enum or typed result variants. Each branch must satisfy
pattern coverage and ownership rules. See [enums](enums.md) and [case and
branch ownership](../ownership/case-and-branch-ownership.md).
