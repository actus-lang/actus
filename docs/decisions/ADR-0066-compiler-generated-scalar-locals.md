# ADR-0066: Compiler-generated Scalar Locals for Role-qualified Calls

- Status: Accepted design; implementation in progress
- Date: 2026-10-05
- Decision owners: Actus language and compiler maintainers

## Context

Actus currently requires a named binding when a role-qualified call argument
needs an intermediate scalar value. This keeps ownership explicit, but it can
make small deterministic expressions unnecessarily repetitive:

```act
erg next: u32 = index + 1u32;
read(value: abs next);
```

The compiler can make this case shorter without hiding ownership transfers if
it materializes only a narrowly defined, pure scalar temporary. This feature
must not become a general temporary-variable generator or hide resource
lifetime, mutation, calls, or borrows.

## Decision

The compiler may normalize an eligible role-qualified call argument before
semantic role validation by inserting a compiler-owned scalar local. The
source language gains no new keyword and the formatter never prints the
generated binding.

The initial accepted form is a direct argument of a call expression that
occupies a statement, declaration initializer, return value, assignment
value, or control-flow condition. The argument must be pure and scalar:

```act
read(value: abs (index + 1u32));
read(value: abs samples[position]);
```

The normalized form is semantically equivalent to:

```act
erg __actus_generated_0: u32 = index + 1u32;
read(value: abs __actus_generated_0);
```

The generated local is compiler metadata, not a user-visible declaration. Its
initializer retains the original argument span for diagnostics and native IR
metadata. Generated locals are scoped to the containing call statement or expression and
cannot be returned, stored in an aggregate, borrowed beyond the call, or
referenced by source code. Nested calls and branch-valued expressions remain
explicit until a later normalization profile defines their evaluation order.

## Eligibility

An expression is eligible only when all of these conditions hold:

- the parameter role is `abs`;
- the inferred result is a known scalar primitive type;
- the expression contains literals, identifiers, grouping, unary operators,
  checked scalar arithmetic, bitwise operators, comparisons, and eligible
  scalar indexing;
- every identifier and index source is read-only for this evaluation;
- every subexpression is evaluated exactly once in source order; and
- no expression can allocate, mutate, move, borrow, call, perform I/O, or
  depend on an aggregate or resource lifetime.

`ins`, `dat`, and `erg` arguments remain place-based and require their current
explicit ownership rules. Buffers, arrays, structs, packs, enums, strings as
owned resources, calls, method calls, `try`, conditionals, case expressions,
assignment, mutation, and borrow expressions are rejected. The explicit
named-binding form remains the escape hatch for every rejected case.

## Evaluation and ownership contract

- Generated locals are materialized in deterministic left-to-right order.
- Generated names are checked against source bindings and remain deterministic.
- Each initializer is evaluated exactly once.
- The generated local receives `erg` ownership for the duration of the call
  expression and is exposed to the callee only through `abs`.
- The local is cleaned up at the end of its call scope using the same scalar
  cleanup rules as an explicit owner.
- A generated local never changes the ownership state of an input binding.
- Native lowering emits scalar computation and the existing call ABI; it must
  not allocate a buffer or introduce a runtime helper symbol.
- Object and executable emission must produce equivalent normalized behavior.

## Diagnostics and tooling

The compiler keeps the original argument span when it rejects an expression or
when a generated initializer fails type, borrow, bounds, or ownership checks.
Diagnostics explain that generated locals are limited to pure scalar
expressions and point users to an explicit binding for unsupported forms.

Formatter, hover, completion, semantic-model, and definition views operate on
the source AST. They must not invent or display generated declarations. Native
IR metadata may identify generated locals for compiler diagnostics and tests.

## Non-goals

- This ADR does not add implicit numeric conversions.
- This ADR does not generate temporaries for resources or aggregates.
- This ADR does not infer `ins`, `dat`, or `erg` ownership from arbitrary
  expressions.
- This ADR does not allow calls, allocation, I/O, or mutation inside an
  eligible generated initializer.
- This ADR does not replace explicit bindings where a stable name or lifetime
  is useful.

## Implementation gates

- [x] Define the eligible expression boundary and rejected categories.
- [x] Define single evaluation and deterministic ordering.
- [x] Define generated-local ownership and scope.
- [x] Preserve explicit bindings as the escape hatch.
- [x] Add semantic normalization and generated-local metadata.
- [x] Add accepted and rejected semantic tests.
- [x] Add native object/executable parity evidence.
- [ ] Add native no-allocation evidence that distinguishes emitted runtime declarations from executed calls.
- [ ] Update the implementation guide after behavior is shipped.
