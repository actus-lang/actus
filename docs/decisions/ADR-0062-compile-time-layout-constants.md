# ADR-0062: Compile-Time Layout Constants

## Status

Accepted for Phase 33, Gate 33.3.

## Context

Protocol and hardware layouts repeat bit offsets, widths, masks, and sentinel
values. Direct numeric offsets are difficult to review and easy to drift when a
layout changes. Actus already has typed compile-time constants and explicit
`pack` storage layouts. The missing contract was allowing a pack field offset
to name one of those constants without turning layout calculation into runtime
work.

## Decision

Pack field offsets accept either the existing non-negative integer literal or a
named compile-time constant:

```act
const HEADER_OFFSET: u16 = 0u16;
const PAYLOAD_OFFSET: u16 = HEADER_OFFSET + 8u16;

pack Frame {
    erg storage: Array[u8, 16];
    layout little;
    fields {
        erg header: u8 at HEADER_OFFSET;
        erg payload: u8 at PAYLOAD_OFFSET;
    }
}
```

The named offset must resolve to a checked, non-negative value representable by
the pack offset domain (`u16`). Constant references may be chained and their
integer arithmetic is evaluated with checked operations. The resolved offset
then goes through the existing pack width, bounds, overlap, reserved-bit, and
full-coverage validation.

The constant evaluator accepts integer literals, named constants, grouping,
integer unary operators, and integer arithmetic/bitwise/shift operators. It
rejects calls, reads, field access, indexing, buffers, strings, booleans,
floating point, runtime state, allocation, and mutation. Cycles and arithmetic
overflow remain semantic errors.

Constants remain compile-time declarations. They produce no runtime storage,
ABI symbol, allocation, or native load. Native lowering receives the resolved
numeric layout value directly. The original constant name remains in the AST
for formatter output, source spans, LSP definition lookup, hover, and semantic
model metadata.

## Compatibility and migration

Existing numeric `at 0` forms remain valid and preserve their formatting. New
layout code should name protocol offsets when the value has a stable meaning.
The named form is intentionally limited to package-visible constant resolution;
dynamic configuration and runtime lengths belong in ordinary runtime code and
cannot define a pack layout.

## Rejected alternatives

- Runtime offset expressions are rejected because native layout must be fixed
  before code generation.
- A separate runtime layout object is rejected because it adds storage and
  obscures the pack ABI.
- Implicit offset numbering is rejected because field placement must remain
  explicit and reviewable.
- A general macro or arbitrary compile-time execution system is rejected because
  the constant contract must remain bounded and deterministic.

## Evidence

- semantic validation accepts named and chained integer offsets;
- runtime and non-integer offset sources are rejected;
- native execution proves the resolved offset is used for pack field access;
- formatter output is idempotent and preserves the named offset;
- LSP semantic metadata and hover retain the offset name;
- object and executable output contain no runtime constant symbol.
