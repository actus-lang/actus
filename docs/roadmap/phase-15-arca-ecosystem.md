# Phase 15: Arca Ecosystem, I/O and Core Runtime

Arca is the planned project and package manager for Actus. It will orchestrate
the compiler, builds, tests, dependency resolution, and package publication.

## Manifest and Project Model

- [x] Define the initial `Arca.toml` manifest schema.
- [x] Define package source roots, unit entry points, and project layout.
- [x] Accept the Directory Module architecture and facade contract in
  [ADR-0018](../decisions/ADR-0018-directory-modules-facade-contracts-and-intra-module-scoping.md).
- [x] Implement the Directory Module Resolver core from an explicit source
  root.
- [x] Resolve both canonical `foo/foo.act` facades and single-file
  `foo.act` modules, rejecting ambiguous roots.
- [x] Discover direct sibling `.act` files deterministically and ignore nested
  directories and non-Actus files.
- [x] Parse facade and sibling sources and aggregate their AST declarations
  into one module scope.
- [x] Validate duplicate struct, enum, role, and verb declarations with both
  source locations.
- [x] Make sibling declarations visible to semantic analysis without imports.
- [ ] Integrate resolver source roots with `Arca.toml` package manifests.
- [x] Validate facade-only `open <sibling>;` exports and zero-import
  intra-module scope.
- [x] Emit compile-time namespace collision diagnostics across siblings.
- [x] Define initial package names, versions, entry points, and Alpha edition.
- [ ] Define the `Arca.lock` dependency lockfile format.
- [ ] Define debug and release profile semantics.

## Local Workflow

- [ ] Add `arca init` and `arca new`.
- [ ] Add `arca check`.
- [ ] Add `arca build` and `arca build --release`.
- [ ] Add `arca run`.
- [ ] Add `arca test`.
- [ ] Add `arca fmt` integration.

## Dependencies and Publishing

- [ ] Define dependency resolution and version constraints.
- [ ] Define local path dependencies.
- [ ] Define registry and package index behavior.
- [ ] Define package validation and reproducible archives.
- [ ] Define package signing and trust policy.
- [ ] Add `arca publish` after the registry contract is stable.
- [ ] Add package download and cache management.
