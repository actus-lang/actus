# ADR-0097: Explicit checked native representations

- Status: Proposed
- Date: 2026-11-01
- Scope: type metadata, native ABI validation, layout safety, diagnostics, and
  formatter/LSP preservation

## Context

Actus needs explicit contracts for value layouts that cross native boundaries.
An ordinary Actus struct is not automatically a foreign ABI contract: it may
contain ownership roles, opaque runtime values, generic parameters, or nested
types whose layout is not fixed. Treating every struct as a native aggregate
would make ownership and layout implicit.

## Decision

Actus supports the closed metadata forms `meta repr(C)`,
`meta repr(packed)`, `meta repr(transparent)`, and the grouped spelling
`meta repr(C, packed)`. The representation names are bare compiler-defined
identifiers, not user strings. Separate `repr(C)` and `repr(packed)` attributes
are combined component-wise; repeating a component is rejected. Transparent is
not composable with C or packed. `repr(packed)` and `repr(transparent)` are
valid only for concrete structs. `repr(C)` remains valid for concrete structs
and unit-variant enums.

The compiler validates the complete declaration before code generation:

1. generic declarations are rejected;
2. struct fields must be value fields, never `erg`, `abs`, or `ins` fields;
3. fields must use supported fixed-size scalar, array, or nested `repr(C)`
   types;
4. nested aggregate contracts must be explicit and recursive layouts are
   rejected;
5. enum payloads, opaque values, packs, handles, and ownership-qualified
   references are rejected;
6. field order and the existing neutral layout engine define padding,
   alignment, size, and offsets; no packing is inferred for `repr(C)`;
7. `repr(packed)` gives every field one-byte alignment, removes inter-field and
   trailing padding, and lowers unaligned field access with an explicit
   one-byte memory alignment contract;
8. packed enums and payload/tagged-union representations are rejected until a
   separate enum ABI contract exists.
9. `repr(transparent)` requires exactly one ordinary value field, uses that
   field's size and alignment at offset zero, and recursively resolves nested
   transparent wrappers. It is rejected for enums, generic declarations,
   ownership-qualified fields, opaque values, recursive layouts, and any
   combination with C or packed representation.

Neither representation grants native access, changes ownership, alters
cleanup, serializes a value, or weakens unsafe-boundary checks. A packed C
aggregate is still an explicit FFI contract: the foreign side must use a
matching packed layout. Packed access is byte-aligned at the backend boundary;
it is not implemented through an assumed naturally aligned pointer.
Transparent representation likewise does not grant native access; it only
asserts that the wrapper is ABI-equivalent to its one checked value field.

## Alternatives considered

- Treat every struct as C-compatible: rejected because it hides ownership and
  makes opaque or generic state cross native boundaries accidentally.
- Accept `repr("C")`: rejected because representation names form a closed
  compiler vocabulary and should not look like arbitrary runtime strings.
- Infer packing from field order or target ABI: rejected because padding must be
  an explicit, auditable declaration choice.
- Use a parser-only marker: rejected because a marker without semantic layout
  validation would provide false ABI confidence.

## Consequences

Users get an explicit, auditable opt-in for C-compatible value aggregates.
Invalid ownership and layout combinations fail before code generation with a
stable diagnostic. Existing declarations remain unchanged because the feature
is opt-in. The cost is that native interoperability requires deliberate type
design and explicit nested annotations.

### FFI aggregate mapping

The program-aware FFI mapper is part of the same checked contract. It resolves
only concrete declarations with `repr(C)`, computes field offsets and padding
for an explicit `CAbiTarget`, and preserves Actus ownership at the boundary:

- aggregate parameters are pointer-passed and retain their declared ownership;
- owned aggregate returns use a caller-allocated return contract;
- borrowed aggregate returns remain borrowed and are never converted to an
  owned native value;
- fixed-width integers, floating-point scalars already accepted by `repr(C)`,
  `Bool`, `Usize`, and positive concrete arrays receive target-aware layouts;
- unannotated, generic, recursive, opaque, ownership-bearing, or unsupported
  fields fail before native lowering.
- packed aggregates report `packed: true`, one-byte aggregate alignment, and
  exact unpadded field offsets; the mapper does not silently invent a foreign
  compiler's packing pragma.
- transparent aggregates resolve directly to the mapped type of their one
  value field, preserving direct scalar/array passing and avoiding an implicit
  aggregate pointer layer.

The convenience program-aware API uses the repository's 64-bit target model.
Callers targeting another pointer width must use the explicit target API. The
mapper reports a checked signature; it does not infer a native symbol, invent
ownership, or bypass the unsafe external boundary.

## Evidence required

The implementation must retain parser positive/negative fixtures, semantic
ownership and layout rejection fixtures, formatter/LSP preservation coverage,
and native layout tests for concrete scalar, array, nested-struct, and unit-enum
cases. Cross-target native ABI claims require target-specific build evidence;
the metadata itself does not claim that every platform uses identical C ABI
calling conventions.
