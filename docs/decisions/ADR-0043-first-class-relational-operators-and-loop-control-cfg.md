# ADR-0043: First-Class Relational Operators and Loop-Control CFG

- Status: Accepted
- Date: 2026-09-29
- Scope: Relational expressions, nested loop control-flow lowering, and length-aware Buffer console output

## Context

Actus currently tokenizes relational punctuation, but the language contract must
make comparison expressions first-class across the complete compiler pipeline.
Loop exits must also retain their enclosing loop target when a `case` branch is
lowered inside that loop. Finally, Buffer output is a counted byte operation;
requiring a C-string terminator would impose an unrelated representation rule.

## Decision

### Relational expressions

`<`, `<=`, `>`, and `>=` are binary operators that return `Bool`. They have
lower precedence than arithmetic operators and require compatible operand
families: matching signed integers, matching unsigned integers, or matching
width floating-point values. Signed and unsigned values are never silently
mixed.

Native lowering selects Cranelift signed integer conditions (`slt`, `sle`,
`sgt`, `sge`) for `Int` and signed primitives, unsigned conditions (`ult`,
`ule`, `ugt`, `uge`) for unsigned primitives and `Usize`, and floating-point
conditions for `f32` and `f64`.

### Loop-control CFG

The active loop target is propagated through nested lexical blocks and `case`
branches. A `break` or `continue` inside a case branch therefore targets the
nearest enclosing loop's exit or header block, including its carried values and
cleanup plan. A case branch never becomes an accidental loop boundary.

### Length-aware Buffer output

`print(abs text: Buffer)` and the line-buffer variants pass the live Buffer
handle to a counted runtime ABI. The runtime validates the handle and writes
exactly `length` bytes. Buffer output does not inspect or require a trailing
null byte; C-string APIs remain separate and are used only for String values.

## Invariants

1. Every accepted relational expression has result type `Bool`.
2. Operand signedness, width, and float width are validated before codegen.
3. Loop exits resolve to the nearest active loop, even across `case` blocks.
4. Buffer output never reads beyond `length` and never requires `capacity > length`.
5. Runtime status values remain explicit ABI statuses or exact byte counts.

## Consequences

Relational control flow can be used directly in guards and loop state without
integer truthiness conventions. The compiler must carry loop-target context
through expression lowering. Buffer values may contain arbitrary non-zero and
zero bytes and still print deterministically according to their live length.

## Non-Goals

This ADR does not introduce equality operators, implicit numeric conversions,
filesystem path resolution, or C-string semantics for Buffer values.
