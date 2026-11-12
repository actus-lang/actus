# ADR-0106: Source Layout and Module Facades

## Status

Proposed

## Context

The compiler source has grown through several valid feature additions, but a
few directories now contain both a module facade and implementation logic.
Some module names also describe implementation convenience rather than the
domain they own. This makes ownership of code, unit tests, and public exports
harder to audit.

## Decision

- A directory is a Rust module. Its root is `mod.rs`.
- `mod.rs` is a facade: documentation, `mod`/`pub mod` declarations,
  attributes, `use`, and `pub use` are allowed; implementation items belong in
  responsibility-specific files.
- `X.rs` and `X/` must not coexist. If `X.rs` has logic, that logic moves to
  a named responsibility file and `X/mod.rs` becomes the facade. An existing
  facade may become `X/mod.rs` directly.
- A submodule is created only when at least two files belong to the same
  domain family and their relationship is supported by imports or use sites.
  A one-file directory is not created.
- The following names are forbidden for Rust source files: `utils`, `common`,
  `helpers`, `misc`, `support`, `implementation`, `entry`, `core`, `base`,
  `impl`, and `stuff`. A file must not repeat its parent directory name, and
  Rust keywords are not used as file names.
- Private unit tests in `src/` live in `tests.rs` in the owning module
  directory. Integration tests remain in `tests/` and mirror the public
  module boundary.
- Inline `#[cfg(test)] mod tests { ... }` blocks are allowed in ordinary
  implementation files, following Rust convention. A separate `tests.rs` is
  used when tests are large or cover multiple files in one module. Inline
  tests are not allowed in facade `mod.rs` files.
- Existing public paths are preserved through deliberate `pub use` facade
  exports.
- This decision changes only source layout, module paths, and imports. Logic,
  ABI, diagnostics, snapshots, and command output must remain unchanged.

## Consequences

The migration is staged by module family. Each migration records the old to
new path map, preserves test names where possible, and verifies that source
logic is unchanged. The architecture conformance test uses an explicit,
shrinking allowlist for violations that exist before each migration PR. The
allowlist was emptied and removed in Phase 52 PR 10; all rules now apply
without exceptions.

