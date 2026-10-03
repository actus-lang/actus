# ADR-0054: Package Configuration and Compile-Time Modules

- Status: Accepted as the Phase 25 contract
- Scope: package configuration facade, compile-time declarations, module
  visibility, dependency direction, and native lowering boundary
- Depends on: ADR-0047 and Phase 24 hierarchical facades

## Context

Actus packages need a stable location for typed policy values shared by domain
modules. Keeping values inside one implementation module makes ownership of
the package contract unclear. Treating configuration as an ordinary runtime
module permits accidental cycles, runtime state, and ABI symbols for values
that should exist only during compilation.

Phase 24 already provides canonical directory facades and parent-controlled
exports. Phase 25 applies that model to a reserved package configuration
boundary.

## Decision

A package may define one canonical configuration facade at:

```text
src/config/config.act
```

The `config` module is a reserved package-level identity. Its dependency
direction is one-way:

```text
package modules -> config facade -> config siblings
```

Configuration sources may expose typed compile-time constants and supporting
type-level declarations required to describe them. They may not contain
runtime verbs, mutable owners, ownership roles, external imports, or runtime
dependent initializers.

The configuration facade may explicitly open configuration siblings or nested
configuration facades. External package code may write `import config;`, but a
configuration source may not write any `import` declaration. Direct child
paths bypassing `config/config.act` remain invalid.

## Public interface

```act
// src/config/config.act
open values;
open limits;
```

```act
// src/config/values.act
open const DEFAULT_THRESHOLD: u8 = 30u8;
open const BRAIN_VERSION: u16 = 1u16;
```

```act
// package source
import config;

verb threshold() -> u8 {
    return DEFAULT_THRESHOLD;
}
```

Configuration constants are compile-time bindings. They have no runtime
storage, callable identity, exported ABI symbol, or generated function.

`Actus.toml` remains responsible for package, build, target, runtime, and
deployment configuration. Source configuration does not replace manifest
configuration and must not be used to smuggle runtime environment values into
compile-time expressions.

## Required diagnostics

The compiler must reject, deterministically and before native emission:

- an `import` inside the configuration subtree;
- a runtime verb, mutable owner, or ownership role inside configuration;
- a direct import that bypasses the canonical configuration facade;
- a missing or ambiguous `config/config.act` facade;
- a private constant used outside its configuration module;
- duplicate public constant exports;
- configuration dependency cycles;
- runtime-dependent, overflowing, or type-incompatible initializers.

The semantic analyzer and native backend must consume the same public
configuration interface. A value visible to `actus check` must either be
available to native lowering as a validated compile-time binding or produce a
consistent diagnostic; it must never degrade into an undeclared runtime
identifier.

## Consequences

Positive consequences:

- package-wide policy values have one explicit, reviewable owner;
- configuration cannot depend on runtime or domain modules;
- compile-time constants remain free of storage and ABI overhead;
- nested configuration values follow the existing facade and visibility rules;
- cache and native output identity can include a canonical configuration
  interface fingerprint.

Costs:

- the compiler must support const-only module units without anchor verbs;
- resolver, semantic analysis, codegen, LSP, formatter, and test runner need
  one shared configuration contract;
- configuration cycles and public export collisions need dedicated diagnostics.

## Non-goals

This ADR does not introduce mutable globals, runtime dependency injection,
implicit imports, wildcard exports, target-specific configuration syntax, or
AIE-specific language constructs.

## Contract fixtures

The positive and negative layouts in
`tests/fixtures/configuration/phase-25-contract.txt` are contract fixtures.
They are intentionally recorded before implementation so parser, resolver,
semantic, native, and tooling gates can all test the same boundary.
