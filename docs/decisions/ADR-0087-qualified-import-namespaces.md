# ADR-0087: Qualified Import Namespaces

- Status: Accepted
- Date: 2026-10-24
- Scope: Actus module resolution, name lookup, diagnostics, and tooling

## Context

Actus currently uses `import` to make a module facade available and `open` to
bring declarations from that facade into the current unqualified namespace.
This makes a source file's dependencies less visible and increases the chance
of collisions when several modules export similarly named constants, structs,
or verbs.

Large Actus packages need a stable, reviewable boundary: the call site should
show which module owns a symbol without requiring readers to inspect every
`open` declaration at the top of the file.

## Decision

Add qualified module-namespace access as the preferred import style:

```actus
import config;

config::MAX_VALUE
```

`import module;` binds the canonical facade as a module namespace. Qualified
lookup may access only declarations exported by that facade and its approved
public surface. A qualified lookup must not bypass a parent facade or expose a
private implementation child.

For an external consumer, standalone `open module;` is not permitted. An
external consumer must place the opening explicitly inside the unified import
block:

```actus
import {
    driver::{i2c, uart},
    telemetry::uart as telem_uart,
    open sensors::imu,
    open config::{MAX_SPEED, MIN_VOLTAGE},
};
```

`driver::{i2c, uart}` binds the imported namespaces locally so they are
accessed as `i2c::...` and `uart::...`. The `as` clause is strictly a
module/namespace alias and cannot rename individual declarations.

A directory facade is a different scope. Any `open` declaration written in a
directory facade means that the facade is re-exporting that public surface to
its consumers. This applies to every facade declaration, not only to examples
such as `open reader;` or `open writer;`. Such facade `open` declarations
remain valid so the canonical facade architecture is preserved. The compiler
must keep external imports and internal facade re-exports distinct:

- `import` establishes a module binding and does not inject names into the
  surrounding namespace;
- in external consumer scope, `open` inside an import block explicitly adds
  the selected facade's public declarations to the unqualified lookup scope;
- in directory facade scope, `open` declares the public re-export surface of
  that facade and is not an external consumer import;
- qualified lookup resolves through the imported module binding and never
  depends on ambient `open` state.

The exact token spelling and AST representation must be shared by semantic
analysis, formatter, language server, diagnostics, Cranelift lowering, and
LLVM lowering. No backend may implement a separate qualified-name rule.

### Binding and package-resolution rules

- A plain import binds the final module segment as its local namespace. For
  example, `import driver::i2c;` is referenced as `i2c::...`.
- `as` replaces only that local namespace binding. It is never a declaration
  rename and cannot rename a verb, type, constant, enum variant, or field.
- Grouped imports are normalized to independent namespace bindings, so every
  member of `driver::{i2c, uart}` is checked for its own local binding and
  source span.
- A nested path is resolved one module segment at a time through canonical
  facades. A child is externally importable only when its own canonical facade
  exists and the path does not bypass an existing parent facade.
- A package-root alias selects a configured dependency root; it does not
  permit filesystem paths, arbitrary directory traversal, or direct access to
  a dependency's implementation files. Runtime `std::...` roots retain their
  target-compatibility checks.
- A path that names a non-facade implementation child is rejected before
  semantic lookup. The diagnostic identifies the parent facade that must be
  used instead.
- Repeating any local import binding is an error, including repeating the
  exact same path. Different paths may not share an alias or final namespace
  segment.

## Constraints and invariants

- Canonical facade and private-child visibility rules remain unchanged.
- Qualified access cannot reach private declarations or filesystem paths.
- Duplicate local bindings and ambiguous qualified paths are rejected with
  stable diagnostics before code generation.
- Unused imports and unresolved qualified names remain diagnosable.
- Name resolution remains deterministic and independent of declaration order
  outside the imported facade contract.
- Existing directory-facade `open` declarations remain valid. External
  consumer `open foo;` declarations must migrate to `import { open foo };`;
  migration is not performed by silently rewriting user code.
- LLVM and Cranelift receive the same resolved declaration identity and type.

## Rejected alternatives

### Keep `open` as the only access form

This hides ownership of names at call sites and makes large-module collision
review unnecessarily difficult.

### Automatically open every imported facade

This increases ambient namespace coupling and makes adding a public declaration
to a module a potentially breaking change for unrelated consumers.

### Permit direct imports of implementation children

This bypasses canonical facades, weakens visibility boundaries, and makes
module refactoring part of the public API accidentally.

## Migration and compatibility

The first implementation adds qualified lookup while preserving `open` for
directory-facade re-exports. External consumer imports migrate to the unified
`import { ... };` form. The standard library may later choose the equivalent
`open { ... };` facade spelling, but that is a formatter and migration choice,
not a requirement of this ADR.

## Required evidence

- Parser and AST tests for qualified paths and malformed paths.
- Semantic tests for successful facade lookup, private-child rejection,
  ambiguity, missing modules, and missing declarations.
- Formatter and language-server round-trip coverage.
- Cranelift/LLVM parity tests proving identical resolved symbols and behavior.
- A migration inventory showing which standard-library modules still use
  `open` and why.

## Acceptance evidence

Accepted on 2026-10-24 after compiler revision `7daf9fd7`. Formatter, lockfile,
strict-check, full test, and native backend parity validation passed. The
default Cranelift profile and the LLVM-enabled `--all-features` profile both
passed, including 10/10 qualified backend-parity tests and strict zero-float
native verification. Remaining directory-facade `open` declarations are
intentional public re-exports; they are not external consumer imports and are
outside this migration decision.
