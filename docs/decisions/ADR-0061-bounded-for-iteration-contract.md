# ADR-0061: Bounded `for` Iteration Contract

## Status

Accepted for Phase 33, Gate 33.2.

## Context

Actus already has a verified `loop` construct, but hand-written counter loops
repeat bounds checks, increment logic, and loop control plumbing. A readable
iteration form is useful only if it keeps the same static ownership and cleanup
guarantees. An iterator must not introduce an allocator, an implicit collection,
or an unbounded execution path.

## Decision

Actus supports two bounded forms:

```act
for erg index: u32 in 0u32 .. 8u32 {
    total += index;
}

for erg index: u32 in values {
    total += values[index];
}
```

The range is half-open: `start` is included and `end` is excluded. The range
binding requires an explicit primitive integer type and an explicit ownership
role. The initial profile accepts `erg` only because the binding is a fresh,
mutable scalar that carries the current index. `abs`, `ins`, and `dat` bindings
are parsed but rejected by semantic analysis; they do not silently change the
ownership contract.

The collection form is index iteration over a statically known `Array[T, N]`.
It accepts direct array bindings and pack fields backed by fixed arrays. The
binding type is the integer index type, and the compiler derives the finite
bound `N` from the array layout. Dynamic collections, scalar expressions,
unbounded iterators, and runtime length discovery are rejected.

Empty and reversed ranges execute zero times because the condition is the
typed comparison `index < end`. The compiler requires both range expressions
to match the binding integer type, so a literal that cannot be represented in
that type is rejected before native lowering. The half-open contract also
prevents an increment after the maximum valid endpoint from wrapping into a
second traversal.

`break`, `continue`, nested loops, and early return use the existing loop
cleanup plan. `continue` targets the generated increment block, so it cannot
skip index advancement. Outer mutable scalar bindings are carried through the
same verified block parameters used by `loop`. The iterator itself is a scalar
block value and has no cleanup allocation.

## Lowering and safety constraints

The compiler preserves the source range or collection span in the AST. Native
lowering emits condition, body, increment, and exit blocks. It performs no
hidden allocation and emits integer control flow only. Collection iteration is
lowered to the same bounded range CFG after the fixed array capacity has been
resolved from the layout registry.

The formatter prints the canonical form. Definition and hover lookup expose
the iterator binding in the same way as other local bindings. Invalid roles,
scalar sources, type mismatches, and unsupported dynamic sources produce
deterministic diagnostics.

## Rejected alternatives

- An unbounded `for item in source` form is rejected because it hides termination
  and makes native cost unreviewable.
- A runtime collection length call is rejected in the initial profile because
  it weakens fixed layout reasoning and can introduce hidden work.
- Implicit ownership roles are rejected for iterator bindings. The role is part
  of the source contract even though only `erg` is currently accepted.
- Rewriting `for` into source-level `loop` text is rejected because it loses
  source spans and can make `continue` skip the increment step.

## Evidence required for the gate

- Parser and formatter tests preserve range and collection syntax.
- Semantic tests reject invalid roles, scalar or unbounded sources, and typed
  bound mismatches.
- Native tests cover ranges, reversed ranges, `break`, `continue`, fixed arrays,
  and pack-backed arrays.
- Strict compiler checks and the full test suite show no floating-point or
  allocation path introduced by the iterator lowering.
