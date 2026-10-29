# Phase 48: Meta-attribute contract and extension readiness

This phase establishes a stable, documented contract for Actus metadata before
new attributes are proposed. It does not introduce a new attribute by itself.

## Scope

- `meta test`
- `meta target(...)`
- `meta fault(...)`
- `meta limitless(...)`
- parser, semantic validation, formatter, LSP, test-runner, and diagnostics
  boundaries shared by those attributes

## Work plan

- [x] Inventory every existing metadata form, argument, placement rule, and
      consumer.
- [x] Link each attribute to a focused handbook document under
      `docs/guide/meta-attributes/`.
- [ ] Accept ADR-0089 after implementation and compatibility review.
- [x] Add the first parser negative fixture for test-only fault metadata.
- [x] Add the remaining parser and semantic negative fixtures for malformed,
      duplicate, and misplaced metadata.
- [x] Add formatter and LSP fixtures that preserve metadata boundaries.
- [x] Verify test-only metadata is unavailable in ordinary production paths.
- [x] Verify target metadata on Unix, POSIX, and Windows target contracts.
- [x] Verify every supported fault with a strict fixture and no invented names.
- [x] Audit `limitless` exceptions against the source-limit policy.
- [x] Record compiler version, platform matrix, strict checks, and unresolved
      limitations before adding new metadata ideas.

## Validation evidence

Recorded on 2026-10-29 at commit `a30da53`.

- Actus package: `0.1.0`.
- Rust toolchain: `rustc 1.98.0 (88d9e12ae 2026-08-18)` and
  `cargo 1.98.0 (797e8a9bc 2026-08-05)`.
- Host validation: Linux `7.2.11-arch1-1`, `x86_64`.
- CI platform coverage: Ubuntu, macOS, and Windows. Release target coverage is
  `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`,
  and `x86_64-pc-windows-msvc`; the CI target contract also includes the
  ARM32 freestanding job.
- Strict validation passed with `cargo check --all-targets --all-features`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and
  `cargo test --all-targets --all-features -- --test-threads=1`.
- Focused validation passed for parser diagnostics, formatter preservation,
  LSP diagnostics, target filtering, source-limit policy, stdlib conformance,
  production code-generation parity, and the complete supported fault fixture
  registry.

Known limitations remain explicit: this phase validates the four existing
metadata attributes only. New attributes still require a separately reviewed
design, implementation, fixtures, and documentation. ADR-0089 remains
`Proposed` until that compatibility review is completed. The local platform
matrix uses target-contract fixtures; native behavior on each release target
is additionally established by the corresponding CI jobs.

## Completion standard

The phase is complete only when the existing attributes have one consistent
documented contract, the relevant fixtures pass, diagnostics are stable, and
the implementation is ready for a separately reviewed proposal for any new
attribute.

