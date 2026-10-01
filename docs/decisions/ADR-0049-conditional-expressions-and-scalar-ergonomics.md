# ADR-0049: Conditional Expressions and Scalar Ergonomics

- Status: Proposed
- Date: 2026-10-01
- Scope: Actus conditional control flow, scalar expressions, typed literals,
  and compound assignment

## Context

Actus currently expresses branching through `case`. This is correct for
matching enum and `Result` variants, but it is unnecessarily verbose for
ordinary boolean decisions. A simple condition should not require a synthetic
boolean match branch:

```actus
case ready {
    true => run(),
    false => wait(),
}
```

The language also makes small scalar operations more explicit than necessary.
Indexed loops and counters should be able to use the declared integer type
directly, and users should not need temporary bindings merely to represent a
typed literal or increment an existing counter.

These issues are related because they affect the everyday expression surface,
not the ownership model or native representation. The solution must preserve
Actus's explicit typing and ownership guarantees while removing ceremony from
simple scalar code.

## Decision

Actus will add a first-class conditional expression and a small, explicitly
typed scalar-ergonomics surface:

1. `if condition { ... } else { ... }` is the ordinary boolean branch form.
2. Integer arithmetic is valid when operand types are compatible, including
   direct operations on `u32` and `Usize` loop indices.
3. Compound assignments such as `index += 1`, `index -= 1`, `value &= mask`,
   and `flags <<= shift` are lowered as checked, typed assignments.
4. Typed numeric literals such as `1u32`, `0u8`, `-1i32`, `1.0f32`, and
   `2.5f64` provide an explicit literal type without a temporary binding.

`case` remains the exhaustive pattern-matching construct for enums, `Result`,
`Option`, and other declared data variants. `if` is not a replacement for
pattern matching and does not introduce implicit truthiness: its condition
must have type `Bool`.

## Conditional expressions

The grammar accepts:

```actus
if condition {
    then_expression_or_block
} else {
    else_expression_or_block
}
```

The `else` branch is required for expression-valued conditionals and optional
for statement-position conditionals whose branches return `Void`. Nested
`else if` forms are syntactic sugar for an `else` branch containing another
`if` expression; the AST may represent them either as nested conditionals or
as an equivalent explicit branch tree, but semantic behavior must be stable.

The condition is evaluated exactly once. The non-selected branch is not
evaluated, and ownership cleanup is planned independently for both branches.
Values produced by both branches must have compatible types. A conditional
may not silently convert between integer widths, signedness, resource roles,
or borrowed and owned values.

The semantic analyzer is responsible for:

- requiring `Bool` conditions;
- checking branch type compatibility;
- validating ownership transitions in each branch;
- joining binding, borrow, and cleanup state at the merge point; and
- rejecting a use after a branch-local move or an ambiguous ownership join.

The code generator lowers the conditional to explicit control-flow blocks.
It must not repair invalid branch types or ownership state. Both branches must
join at one typed result block when the conditional is used as an expression.

## Scalar arithmetic and compound assignment

Actus keeps arithmetic explicit and typed. Compatible integer operands may be
used directly, including a `u32` index:

```actus
erg index: u32 = 0u32;
loop {
    index += 1u32;
}
```

Compound assignment is defined as one semantic operation, equivalent to
computing the binary operation and assigning its result back to the same
place. The left-hand place is evaluated once. This matters for indexed and
field assignments, where re-evaluating the receiver or index could change
ownership or observable behavior.

For integer compound assignment:

- the left and right operands must belong to the same compatible integer
  family under the existing type rules;
- signed and unsigned arithmetic retain their existing overflow and underflow
  contracts;
- checked builds trap or reject the operation according to the existing
  integer diagnostic policy; and
- the assignment must satisfy the existing `erg` mutation and `ins` loan
  rules.

Bitwise compound assignments follow the corresponding bitwise operator's
width and signedness rules. Floating-point compound assignment is limited to
operators already valid for the operand type and does not introduce implicit
integer-to-float conversion.

## Typed numeric literals

Typed literals use a suffix attached directly to the literal:

```actus
0u8
1u32
255u8
-1i32
1.0f32
2.5f64
```

The suffix is part of lexical and parser structure, not an identifier or a
post-hoc semantic guess. The semantic analyzer validates the literal against
the requested width and signedness at compile time. A literal outside its
declared range is rejected with a stable diagnostic.

Unsuffixed literals retain the existing contextual inference rules. If no
context determines a safe type, the compiler must report ambiguity rather
than silently selecting a machine-dependent type. Typed literals do not permit
implicit conversion between unrelated primitive families.

The lexer must preserve the literal's source span, including its suffix, so
diagnostics and LSP ranges point to the complete literal.

## Lowering and runtime contracts

The parser produces dedicated AST nodes for conditional expressions,
compound assignments, and typed literal metadata. These nodes remain
backend-independent.

The semantic layer resolves types, validates branch joins, checks arithmetic
compatibility, and records ownership effects before code generation. Cranelift
lowering then emits:

- conditional branch blocks and typed merge parameters;
- the existing checked integer and floating-point operations;
- one load/compute/store sequence for compound assignment, with the existing
  bounds checks for indexed places; and
- constants with the exact declared machine width.

No C bridge is introduced for these language features. No implicit conversion
or backend-specific repair is permitted.

## Invariants

1. `if` conditions have type `Bool`; Actus does not use implicit truthiness.
2. `case` remains the exhaustive pattern-matching construct.
3. The non-selected conditional branch is never evaluated.
4. Conditional branch results must have compatible types and compatible
   ownership states.
5. Compound assignment evaluates its left-hand place exactly once.
6. Compound assignment cannot bypass `erg`, `abs`, `dat`, or `ins` rules.
7. Typed literals are range-checked at compile time.
8. Unsuffixed literals never trigger an undocumented machine-dependent choice.
9. Integer overflow, underflow, division-by-zero, remainder-by-zero, and
   invalid shift counts retain their existing checked behavior.
10. The lexer, parser, semantic analyzer, code generator, formatter, LSP, and
    diagnostics expose one consistent representation of the new syntax.
11. The implementation must remain within Actus file and function size
    limits; parser, semantic, and codegen responsibilities must stay separate.

## Non-goals

- This ADR does not redesign `case`, enum matching, or `Result` propagation.
- This ADR does not add implicit casts or truthy/falsy values.
- This ADR does not define a general macro system or a C-preprocessor layer.
- This ADR does not change `pack` layout lowering or `ins` ownership
  semantics. Those remain separate systems work with their own contracts.
- This ADR does not remove source-size conformance limits; those are governed
  by ADR-0048.

## Consequences

Positive consequences:

- ordinary branching becomes readable without weakening exhaustiveness rules;
- indexed loops can use native unsigned counters directly;
- small numeric operations require fewer artificial bindings;
- typed literals make width and signedness visible at the call site; and
- ownership and checked arithmetic remain enforced by the compiler pipeline.

Costs and risks:

- the lexer and parser must distinguish literal suffixes from identifiers;
- conditional expressions require explicit control-flow and ownership joins;
- compound assignments add place-expression validation and codegen paths;
- formatter, LSP completion, hover, semantic tokens, and diagnostics must be
  updated together; and
- a poorly designed inference rule could reintroduce implicit conversion, so
  accepted and rejected cases must be tested independently.

## Acceptance criteria

The ADR is ready for implementation closure only when:

- valid and invalid `if/else` programs have parser, semantic, and native
  execution tests;
- branch-local ownership moves and cleanup are tested on both paths;
- `u32` and `Usize` counters support direct arithmetic and compound updates;
- typed integer and floating literals have range and type rejection tests;
- formatter round-trips preserve the new syntax;
- LSP diagnostics and semantic information cover the new nodes; and
- formatting, compilation, Clippy, full tests, source limits, and diff checks
  pass together.
