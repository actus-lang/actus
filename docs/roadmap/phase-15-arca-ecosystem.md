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
- [x] Integrate resolver source roots with `Arca.toml` package manifests,
  including default `src/` and custom `package.source_root` discovery.
- [x] Validate facade-only `open <sibling>;` exports and zero-import
  intra-module scope.
- [x] Emit compile-time namespace collision diagnostics across siblings.
- [x] Add `import <module>;` lexer token and AST declaration.
- [x] Parse extension-free module imports with source spans and diagnostics.
- [x] Resolve imports through the Directory Module Resolver.
- [x] Merge imported facade exports into the semantic namespace.
- [x] Reject imports of closed or unlisted sibling symbols.
- [x] Lower a module-aware CLI build and link imported units.
- [x] Verify a real multi-file executable with a stable exit code.
- [x] Define initial package names, versions, entry points, and Alpha edition.
- [x] Define the `Arca.lock` dependency lockfile format, deterministic package
  ordering, generation, and `LockfileOutOfDate` validation.
- [x] Define debug and release profile semantics and map profiles to Cranelift
  optimization levels.

## Local Workflow

- [x] Add `arca init` and `arca new` with optional Git initialization and
  canonical `.gitignore` generation.
- [x] Add `arca check` with module resolution and semantic validation without
  code generation.
- [x] Add `arca build` and `arca build --release` / `--profile <name>`.
- [x] Add `arca run` with profile selection and program-argument forwarding.
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
