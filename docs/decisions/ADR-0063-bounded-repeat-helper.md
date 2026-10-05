# ADR-0063: Bounded Repeat Helper

## Status

Accepted for Phase 33, Gate 33.4.

## Context

Low-level Actus code often repeats the same bounded operation across fixed
slots. A hand-written sequence makes the source long and increases the chance
that one slot receives different ownership, bounds, or error handling. A helper
must reduce that repetition while keeping the generated operations reviewable.

## Decision

Add a bounded `repeat` statement with an explicit mutable index binding and a
half-open integer range:

```act
repeat erg slot: u32 in 0u32 .. 8u32 {
    total += values[slot];
}
```

The helper expands to the equivalent bounded `for` contract during parsing,
before semantic
ownership and bounds analysis. The expansion preserves the helper span on the
generated range and the body span on each generated operation. The index type,
range types, and `erg` role are explicit. `abs`, `ins`, and `dat` helper
bindings are rejected in the initial profile.

The helper is a readability form only. Formatter output canonicalizes it to
`for`; the AST and native pipeline therefore use one bounded loop contract. It
does not introduce an iterator,
closure, callback, allocation, dynamic length lookup, exception path, or
external operation. The body remains ordinary Actus code, so mutation,
ownership transfer, calls, `break`, `continue`, and errors retain their normal
semantic rules. Recursive helper expansion and nested helper declarations are
rejected; nested bounded loops in the body remain valid.

Native lowering uses the same integer CFG and cleanup path as `for`. Generated
operation order is ascending by the source range. Empty or reversed ranges
execute zero times. A source helper and its explicit `for` equivalent must have
the same semantic model and native object bytes.

## Rejected alternatives

- A callback or closure form is rejected because it hides ownership and call
  boundaries.
- A dynamic collection or runtime length form is rejected because it hides
  termination and native cost.
- Implicit index types or ownership roles are rejected because layout and
  ownership must remain explicit.
- A helper that silently catches errors or converts `dat` to `abs` is rejected.
- General macro expansion is rejected because it would make source spans,
  hygiene, and diagnostics less deterministic.

## Migration

Existing `loop` and `for` forms remain unchanged. Use `repeat` only where the
body is a bounded, uniform operation. Keep explicit control flow when each
iteration has different behavior or when the operation order needs visible
source branches.
