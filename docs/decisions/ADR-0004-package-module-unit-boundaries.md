# ADR-0004: Package, Module, and Unit Boundaries

- Status: Accepted
- Date: 2026-09-22
- Scope: Actus language and Actus package model

## Context

Actus needs separate concepts for a published project, an importable source
namespace, and an independently compiled component. Treating these as the
same concept would make import resolution, visibility, dependency management,
and future compiler decomposition ambiguous.

## Decision

Actus defines three distinct boundaries:

### Package

A `package` is a complete Actus project managed and published by Actus. It has
an `Actus.toml` manifest, package identity, version, dependencies, build
configuration, and one or more declared compilation units.

### Module

A `module` is an importable Actus source namespace. A module may contain one
or more source files, may depend on other modules, and does not need to be an
independently compiled artifact.

Modules are closed by default. Only declarations marked `open` are visible
across module boundaries. Module imports do not automatically import every
declaration into the current scope.

### Unit

A `unit` is an independently compiled Actus component with its own public
interface, dependency metadata, compilation result, and cache boundary. A
unit is the Actus concept closest to a Rust crate, but `crate` is not an
Actus or Actus term.

One package may contain multiple modules and multiple units. A module is not
implicitly promoted to a unit merely because it has its own directory.

## Import Syntax

Actus uses `::` for module and namespace paths:

```act
import system::io;
import protocol::codec as codec;

io::read(buffer);
codec::encode(packet);
```

The final path segment becomes the default namespace name. The `as` clause
assigns an explicit alias. Grouped imports are equivalent to multiple single
imports:

```act
import {
    system::io,
    protocol::codec as codec,
};
```

The `::` separator distinguishes namespaces from member access expressions,
which use `.`. Duplicate namespace names, alias collisions, and circular
imports are compile errors with deterministic diagnostics.

## Unit Declaration

`Actus.toml` is the authoritative source for a package's compilation units.
Directory names alone must not implicitly create units.

```toml
[package]
name = "my_kernel"
version = "0.1.0"

[[unit]]
name = "kernel"
kind = "library"
root = "units/kernel.act"

[[unit]]
name = "boot"
kind = "executable"
root = "units/boot.act"
```

Each unit has a unique name and one root source entry point. A root may be a
single source file or a module directory entry point. Only units declared in
`Actus.toml` participate in the package build graph.

The `units/` directory is a recommended layout convention, not an automatic
discovery rule. The initial unit kinds are `library` and `executable`; new
kinds may be added through a future architecture decision.

## Current Alpha Boundary

The current compiler remains one Rust package. Directories such as
`src/lexer`, `src/parser`, and `src/codegen` are implementation modules, not
independently compiled Actus units. Unit extraction is deferred until the
compiler is self-hosting and an independent compilation boundary is useful.

## Consequences

Explicit manifest declarations make the build graph predictable and prevent
accidental units. The separate terms also allow Actus imports, Actus package
publishing, and future compiler decomposition to evolve independently.
