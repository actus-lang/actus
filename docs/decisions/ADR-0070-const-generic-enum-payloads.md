# ADR-0070: Const-Generic Aggregate Payloads in Native Enums

## Status

Accepted.

## Context

Actus already supports bounded const-generic arrays such as
`Array[u8, 20]`, and it supports `Option[T]` and `Result[T, E]` payloads in
native code. The two capabilities must compose: a value of
`Result[Array[u8, 20], Failure]` must be constructible, returned, matched, and
used in a native `case` branch.

The compiler previously lost the complete payload type while binding a case
pattern. It converted the payload type to a plain name string and then
validated it as if it were a named type. `Array[u8,20]` was consequently
reported as the unknown type `Array[u8,20]`. After semantic validation was
repaired, native layout construction exposed a second gap: array layouts were
not available yet while a specialized enum layout was calculating its payload
size and alignment.

## Decision

Case pattern binding parses and retains the complete payload `TypeName`,
including nested arguments and const-generic capacities. Validation uses the
normal type-reference resolver, and the complete type application is stored in
the binding model for later type and ownership analysis.

The native layout registry resolves an array layout by its registered type
definition when the eager layout vector has not yet been populated. This makes
array size and alignment available while enum payload layouts are constructed,
without changing the enum representation or introducing dynamic allocation.

The public language contract remains unchanged. This is compiler support for an
existing composition of two accepted type features.

## Invariants

- Array capacities remain positive, bounded const-generic values.
- Enum discriminant width, payload offsets, ownership roles, and return ABI do
  not change.
- Array storage remains inline, fixed-size, and position-independent.
- Invalid type applications continue to fail during semantic validation.
- Native output remains subject to the integer-only and zero-float policies.

## Verification

The acceptance fixture `result_case_binding_preserves_const_generic_array_payload`
constructs, forwards, matches, and indexes a `Result[Array[u8, 20], Failure]`
value in native code. It must exit with status 41. Existing generic enum,
array layout, ownership, and source-limit tests remain part of the full suite.

The Actus compiler validation completed with `cargo test --all-targets
--all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`cargo fmt --all -- --check`, the source-limit script, and `git diff --check`.
The corrected compiler was installed with `cargo install --path . --force
--locked`. Twin-e retesting is a separate downstream check; the current
working tree there has an unrelated in-progress observation serialization
change that currently reports `E1027` at `tests/aie_observation.act:121`.
