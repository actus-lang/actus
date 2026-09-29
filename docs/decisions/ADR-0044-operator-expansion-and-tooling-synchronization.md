# ADR-0044: Equality, Logical, Remainder, and Bitwise Operators

- Status: Proposed
- Date: 2026-09-29
- Scope: Equality, remainder, logical short-circuiting, bitwise operations,
  and compiler-tooling synchronization

## Context

ADR-0043 established first-class relational operators and the control-flow
infrastructure required to lower them safely. Actus now needs the remaining
core operator families required for ordinary systems and control-flow code:
equality, remainder, logical composition, and bitwise manipulation.

These operators must be added through the complete one-directional compiler
pipeline. A token or syntax-only implementation is insufficient: accepted
expressions must have explicit semantic rules, deterministic diagnostics,
native lowering, and execution evidence.

The existing language surface also includes checked primitive casts, bounded
array and Buffer indexing, packed-field access, and ownership-aware calls.
Language-server and editor tooling must expose the complete combined surface,
not only the newly added tokens.

## Decision

### Equality and remainder

`==` and `!=` are binary equality operators returning `Bool`. Equality is
defined only for compatible values in the same comparison family. Numeric
values are not implicitly converted to make an equality expression valid.
Structured equality is out of scope unless a type explicitly provides a
future equality contract.

`%` is the remainder operator. It uses the same operand-family rules as
arithmetic operators. Signed remainder follows the target's signed integer
semantics; unsigned remainder uses unsigned arithmetic. Division-by-zero and
remainder-by-zero must produce the existing deterministic runtime failure.

### Logical operators

`&&` and `||` accept only `Bool` operands and return `Bool`. They are lowered
as short-circuit control flow:

- `left && right` evaluates `right` only when `left` is `true`;
- `left || right` evaluates `right` only when `left` is `false`.

`!` accepts only `Bool` and returns the logical negation. Short-circuit
branches must preserve ownership cleanup, loop targets, case context, and
expression result flow exactly as ordinary CFG branches do.

### Bitwise operators

`&`, `|`, and `^` are integer-only binary operators. `~` is an integer-only
unary operator. Their result type is the validated common integer type under
the existing explicit-width rules. Signedness is never silently changed.

`<<` and `>>` are integer shift operators. The left operand determines the
result family; the right operand must be an unsigned integer shift count. The
semantic layer must validate the shift count contract, and native lowering
must use deterministic behavior for counts outside the destination width.
The exact invalid-count policy must be represented by stable diagnostics or a
documented runtime trap before the gate is closed.

### Precedence and associativity

The parser must preserve this precedence order, from tighter to looser:

1. unary `!`, `~`, and existing unary operators;
2. multiplicative `*`, `/`, `%`;
3. additive `+`, `-`;
4. shifts `<<`, `>>`;
5. bitwise `&`;
6. bitwise `^`;
7. bitwise `|`;
8. relational `<`, `<=`, `>`, `>=`;
9. equality `==`, `!=`;
10. logical `&&`;
11. logical `||`.

Binary operators remain left-associative unless a later language decision
explicitly defines another associativity. Parentheses always override the
precedence table.

### Native lowering and diagnostics

The semantic analyzer validates operand families before code generation.
Cranelift lowering selects signed or unsigned integer operations according to
the validated type, emits native remainder operations with zero-divisor
protection, and lowers logical operators to explicit short-circuit blocks.
No backend phase may repair an invalid operator expression.

Diagnostics remain renderer-independent and use stable `E####` codes for
invalid operand families, invalid shift counts, and division/remainder by
zero where a compile-time failure is provable.

### Tooling synchronization

The final gate updates all tooling repositories and metadata together. LSP,
the VS Code extension, and Tree-sitter must recognize and present:

- all relational operators from ADR-0043;
- checked casts and unsigned indexing from ADR-0042 Gate 8;
- bounded arrays, indexed slots, and pack-field access from ADR-0042 Gates 1–7;
- the equality, remainder, logical, and bitwise operators from this ADR.

Synchronization includes token classification, syntax highlighting, parser
precedence, completion, hover, formatting, diagnostics, and semantic-token
behavior wherever each external repository supports the capability. The
compiler remains the source of truth; tooling must not invent syntax or
semantic rules that the compiler rejects.

## Invariants

1. Every accepted equality and logical expression has result type `Bool`.
2. Equality never relies on an implicit numeric conversion.
3. Logical right-hand operands are not evaluated when short-circuiting skips
   them.
4. Remainder is never lowered without a deterministic zero-divisor policy.
5. Bitwise operations preserve the validated integer family and width.
6. Shift counts are validated before native code generation or trapped by an
   explicit documented runtime contract.
7. Short-circuit CFG paths preserve ownership cleanup and loop-control state.
8. LSP, VS Code, and Tree-sitter expose only syntax accepted by the compiler.
9. Existing ADR-0042 and ADR-0043 behavior remains regression-tested.

## Consequences

Actus gains complete boolean composition and low-level integer manipulation
without introducing implicit conversions or backend-specific semantics. The
compiler must maintain more CFG state for short-circuit expressions, and the
tooling repositories must be updated in a coordinated final gate.

## Non-Goals

This ADR does not define overloaded operators, user-defined equality contracts,
arbitrary structured equality, implicit truthiness of integers, floating-point
bitwise operations, or a new ownership model.

## Gate 5 Evidence

Gate 5 is complete on the implementation branch. Runtime remainder and shift
failures use deterministic native traps for values that are known only at
execution time. Statically provable cases are rejected before code generation
with stable diagnostics `E1092` (constant remainder by zero) and `E1093`
(constant shift count outside the destination width). The diagnostics model is
tested through plain, colored, and JSON renderers without coupling semantic
analysis to terminal output.

The native suite covers integer and floating-point equality, remainder,
logical short-circuiting, bitwise operations, shifts, runtime traps, ownership
restoration, cleanup, and deterministic repeated object emission. The complete
quality command set passed: `cargo fmt --all -- --check`, `cargo check --all-targets
--all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`cargo test --all-targets --all-features --no-fail-fast`,
`scripts/check_source_limits.sh`, and `git diff --check`.
