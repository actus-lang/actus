# Phase 15: Arca Ecosystem, I/O and Core Runtime

Arca is the planned project and package manager for Actus. It will orchestrate
the compiler, builds, tests, dependency resolution, and package publication.

## Manifest and Project Model

- [x] Define the initial `Arca.toml` manifest schema.
- [x] Define package source roots, unit entry points, and project layout.
- [x] Accept the Directory Module architecture and facade contract in
  [ADR-0018](../decisions/ADR-0018-directory-modules-facade-contracts-and-intra-module-scoping.md).
- [ ] Implement the Directory Module Resolver from `Arca.toml` source roots.
- [ ] Validate the canonical `foo/foo.act` facade and facade-only `open`
  exports.
- [ ] Implement deterministic sibling discovery and zero-import intra-module
  scope with compile-time collision diagnostics.
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
