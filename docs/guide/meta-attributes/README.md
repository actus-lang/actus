# Meta attributes

Meta attributes are compiler-facing declarations written with `meta`. They
describe how Actus should discover, select, validate, or measure a declaration;
they are not runtime values and do not add hidden runtime behavior.

This directory documents the currently supported attributes:

- [`meta test`](test.md): marks a verb for the Actus test runner;
- [`meta target(...)`](target.md): selects declarations for a supported target
  family;
- [`meta fault(...)`](fault.md): injects one controlled failure into a strict
  test fixture;
- [`meta limitless(...)`](limitless.md): records a narrowly scoped exception
  to source-size conformance limits.
- [`meta must_use(...)`](must-use.md): requires an explicit use of a marked
  verb result.
- [`meta deterministic`](deterministic.md): rejects filesystem, network, and
  entropy effects from deterministic verbs.

[`meta effect(...)`](effect.md) declares compiler-checked execution effects
such as blocking, filesystem, network, allocation, and entropy.

## General rules

- Metadata must use a compiler-recognized attribute name and argument shape.
- Metadata is checked before code generation; it cannot bypass parsing,
  semantic analysis, ownership checks, ABI validation, or runtime safety
  contracts.
- Placement is part of the contract. An attribute may be valid before a verb,
  in a file prelude, or only in a test fixture, depending on its definition.
- Unknown attributes, unsupported values, duplicate attributes, and invalid
  placement are compile-time errors.
- New metadata requires parser, semantic, formatter, tooling, documentation,
  and positive/negative regression evidence.

The language does not treat arbitrary `meta` names as user-defined annotations.
Adding a new attribute is a compiler change with an explicit compatibility and
diagnostics contract.

