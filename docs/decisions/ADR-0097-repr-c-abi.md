# ADR-0097: Explicit checked C representation

- Status: Proposed
- Date: 2026-11-01
- Scope: type metadata, native ABI validation, layout safety, diagnostics, and
  formatter/LSP preservation

## Context

Actus needs a way to identify a value type that is intentionally compatible
with a C-style native aggregate. An ordinary Actus struct is not automatically
a foreign ABI contract: it may contain ownership roles, opaque runtime values,
generic parameters, or nested types whose layout is not fixed. Treating every
struct as a C aggregate would make ownership and layout implicit.

## Decision

Actus supports the closed metadata form `meta repr(C)`. The representation name
is a bare compiler-defined identifier, not a user string. The attribute is
valid only on concrete `struct` and unit-variant `enum` declarations.

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
   alignment, size, and offsets; no packing is inferred.

`repr(C)` does not grant native access, change ownership, alter cleanup,
serialize a value, or weaken unsafe-boundary checks. `repr(packed)`, volatile
access, explicit alignment, and tagged-union layouts are separate future
contracts and are not accepted spellings of this attribute.

## Alternatives considered

- Treat every struct as C-compatible: rejected because it hides ownership and
  makes opaque or generic state cross native boundaries accidentally.
- Accept `repr("C")`: rejected because representation names form a closed
  compiler vocabulary and should not look like arbitrary runtime strings.
- Add `repr(packed)` at the same time: rejected because unaligned access and
  target-specific ABI rules require a separate safety contract.
- Use a parser-only marker: rejected because a marker without semantic layout
  validation would provide false ABI confidence.

## Consequences

Users get an explicit, auditable opt-in for C-compatible value aggregates.
Invalid ownership and layout combinations fail before code generation with a
stable diagnostic. Existing declarations remain unchanged because the feature
is opt-in. The cost is that native interoperability requires deliberate type
design and explicit nested annotations.

## Evidence required

The implementation must retain parser positive/negative fixtures, semantic
ownership and layout rejection fixtures, formatter/LSP preservation coverage,
and native layout tests for concrete scalar, array, nested-struct, and unit-enum
cases. Cross-target native ABI claims require target-specific build evidence;
the metadata itself does not claim that every platform uses identical C ABI
calling conventions.
