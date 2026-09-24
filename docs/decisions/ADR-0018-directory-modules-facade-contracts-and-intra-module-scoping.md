# ADR-0018: Directory Modules, Facade Contracts, and Intra-Module Scoping

- Status: Accepted
- Date: 2026-09-25
- Scope: Actus module resolution and Arca package source layout

## Context

Actus modules must be easy to split across focused source files without
creating unnecessary internal import graphs. At the same time, external API
discovery must remain explicit and deterministic. Package, module, and unit
remain separate concepts as defined by ADR-0004; this decision defines the
source layout and resolver rules for directory-backed modules.

## Decision

### Directory module and canonical facade

A directory is one cohesive Actus module namespace within its enclosing Arca
unit. A multi-file module must contain a root entry file whose name exactly
matches the directory name:

```text
src/
└── driver/
    └── gpio/
        ├── gpio.act       // canonical facade for driver::gpio
        ├── registers.act  // internal declarations
        ├── roles.act      // internal role declarations
        └── ops.act        // internal verb implementations
```

For module path `driver::gpio`, Arca resolves the directory and requires
`driver/gpio/gpio.act` as its canonical entry point. A missing or ambiguous
facade is a compile-time module resolution error.

The facade is the only external API gateway. Only declarations marked `open`
in `gpio/gpio.act` are visible to importers. An `open` declaration in a
sibling file is visible within the module but is not exported through the
module boundary unless the facade explicitly exposes it.

Facade re-exports use an extension-free sibling declaration:

```act
open registers;
open ops;
```

`open <sibling>;` resolves only to a direct `<sibling>.act` file in the same
directory. The facade re-exports only declarations marked `open` inside those
named siblings. Closed sibling declarations remain private, and an unlisted
sibling contributes no external symbols even if it declares `open` items.
The facade contains no implementation declarations; it is only a gateway for
sibling exports. An unknown sibling name is a deterministic
`UnknownSiblingModule` compile-time error.

### Zero-import sibling scope

All `.act` files directly inside the module directory share one internal
scope. Sibling declarations may refer to one another without `import`
statements, relative paths, or re-export boilerplate. This includes internal
structs, enums, roles, type bounds, and helper verbs.

Sibling files are compiled together as part of the enclosing unit. A
declaration name may occur only once in that shared namespace; duplicate
types, roles, enums, or verbs are deterministic compile-time errors. A nested
directory begins a separate module and is not implicitly included in its
parent.

### Deterministic discovery

Discovery is independent of operating-system directory enumeration order. The
resolver must:

1. resolve the module root from the `Arca.toml` package source root;
2. verify the exact canonical facade filename;
3. collect only direct `.act` children of the module directory;
4. sort source paths by normalized relative path before parsing;
5. reject duplicate declarations with stable source locations; and
6. produce identical namespace resolution, diagnostics, metadata, and build
   artifacts for identical source trees.

The filesystem never determines semantic declaration order. Source ordering
is an implementation detail after deterministic discovery and must not change
name resolution or generated output.

### Package and unit boundary

Arca remains authoritative for package source roots and unit entry points.
Directory discovery must not create an independent unit or bypass the
`Arca.toml` build graph. A module may be part of a unit, while a unit may
contain multiple modules. Independent binary linking still follows the unit
metadata and ABI rules in ADR-0006 and ADR-0017.

## Diagnostics

The resolver must provide deterministic diagnostics for:

- missing canonical facade files;
- multiple candidate facades;
- source files found outside the declared module root;
- duplicate declarations across sibling files; and
- nested modules referenced without an explicit module path.

Diagnostics must identify the module path and the relevant source paths. The
semantic analyzer, not the lexer or parser, owns namespace collision and
visibility validation.

## Consequences

- Internal code can be split by responsibility without import boilerplate.
- The facade makes the public surface local, reviewable, and predictable for
  developers and language tooling.
- Deterministic discovery prevents host filesystem behavior from affecting
  builds or diagnostics.
- Arca must implement module-root resolution before directory modules can be
  used by package builds.
- Zero-import scope is intentionally limited to direct siblings; it does not
  weaken package, unit, or external visibility boundaries.

## Implementation Gate

This ADR records the accepted architecture. The Directory Module Resolver,
facade export validation, and sibling discovery remain Phase 15
implementation tasks and are not implied to exist in the current compiler.
