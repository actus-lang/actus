# ADR-0009: Arca Build Graph and Generated Artifacts

- Status: Accepted
- Date: 2026-09-22
- Scope: Arca dependency resolution, unit builds, caching, and artifacts

## Context

Arca needs a deterministic build graph for packages, units, and modules. It
must resolve dependencies reproducibly, reject incompatible precompiled units,
and keep generated artifacts separate from source files.

The word `target` is reserved for a compilation target: architecture, ISA,
environment, ABI, and related platform requirements. Generated build files
must use a different directory name.

## Decision

Arca models builds through three levels:

```text
package
└── unit
    └── module
```

- A package is the distribution and dependency boundary.
- A unit is the independently compiled component.
- A module is the source namespace within a unit.

The package manifest declares build intent. The lockfile records the exact
resolved graph. Unit metadata validates compatibility. The cache reuses only
exactly matching inputs. Cyclic dependencies are rejected.

## Dependency Declaration

Package dependencies are declared in `Arca.toml`:

```toml
[package]
name = "sensor_app"
version = "0.1.0"

[dependencies]
hal = "0.3.0"
serial = { version = "1.2.0", source = "registry" }
```

When a package contains multiple independently compiled units, unit-level
dependencies may be declared explicitly:

```toml
[[unit]]
name = "app"
kind = "executable"
root = "units/app.act"
dependencies = ["hal", "serial"]
```

The manifest is the authoritative source for declared dependencies and units.

## Dependency Resolution

Arca resolves:

- package versions and sources;
- required units;
- target and operating profile;
- unit interface compatibility;
- transitive dependencies.

Package and unit dependency cycles are invalid:

```text
package A -> package B -> package A
unit A -> unit B -> unit A
```

Cycle diagnostics must be deterministic and identify the complete cycle.

## Lockfile

`Arca.lock` records the exact dependency solution, including:

- resolved package versions;
- package sources and content hashes;
- the resolved dependency graph;
- selected units;
- compiler and toolchain identity;
- compilation target and profile;
- relevant ABI metadata.

`Arca.toml` expresses version and configuration requirements. `Arca.lock`
records the exact reproducible resolution used for a build.

## Cache Identity

A cached unit is reusable only when all relevant inputs match. The cache key
must include:

```text
package source hash
unit source hash
dependency interface hashes
compiler version
toolchain hash
target triple
profile
ABI identity
build settings
```

Any change to one of these inputs invalidates the affected unit and its
dependent artifacts.

## Generated Artifact Directory

All generated build files are stored under `capsula/`. The name `target` is
not used for this directory because it is reserved for compilation target
profiles.

The initial layout is:

```text
capsula/
├── debug/
│   └── <target-triple>/
│       ├── <unit>.actmeta
│       ├── <unit>.obj
│       └── <unit>.bin
├── release/
└── cache/
```

`capsula/` is entirely generated and must not be committed to version
control. Debug and release artifacts are separated, and target-specific
artifacts are isolated by target triple.

## Precompiled Units and Source Fallback

Arca may reuse a precompiled unit only when its interface metadata and cache
identity match the current build.

When metadata is incompatible:

1. Arca rejects the stale binary before linking;
2. if compatible source is available, Arca recompiles the unit;
3. Arca regenerates metadata and artifacts;
4. linking continues only after compatibility checks pass.

If source is unavailable, the build fails with a deterministic diagnostic.
An incompatible precompiled unit must never be linked silently.

## Reproducibility

The same source, manifest, lockfile, compiler, toolchain, target, profile, and
build settings must produce the same resolved graph and equivalent generated
artifacts. Artifact naming and metadata emission must be deterministic.

## Consequences

Arca gains an explicit and reproducible package build model. `capsula/`
separates generated state from source and removes ambiguity between artifact
storage and compilation targets. Exact cache identity and source fallback
prevent stale or incompatible units from entering a build silently.

