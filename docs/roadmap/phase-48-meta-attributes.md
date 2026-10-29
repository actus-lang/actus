# Phase 48: Meta-attribute contract and extension readiness

This phase establishes a stable, documented contract for Actus metadata before
new attributes are proposed. It does not introduce a new attribute by itself.
The extension backlog below is intentionally recorded here so that future
metadata work can be executed in a deliberate order without closing this
phase prematurely.

## Scope

- `meta test`
- `meta target(...)`
- `meta fault(...)`
- `meta limitless(...)`
- `meta must_use(...)`
- `meta effect(...)`
- `meta deterministic`
- `meta bounded(...)` with explicit iteration, call, allocation, stack, and I/O
  budgets
- parser, semantic validation, formatter, LSP, test-runner, and diagnostics
  boundaries shared by those attributes

## Work plan

- [x] Inventory every existing metadata form, argument, placement rule, and
      consumer.
- [x] Link each attribute to a focused handbook document under
      `docs/guide/meta-attributes/`.
- [x] Accept ADR-0089 after implementation and compatibility review.
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
- [x] Implement `meta must_use(...)` with its focused ADR, guide, diagnostics,
      formatter/LSP coverage, imported-signature propagation, and semantic
      fixtures.
- [x] Implement `meta deterministic` with an explicit forbidden-effect
      contract, stable diagnostics, formatter/LSP coverage, and semantic
      fixtures.
- [x] Implement `meta realtime` with forbidden-effect, bounded-loop,
      floating-point, recursion, role-contract, formatter, LSP, and semantic
      fixture coverage.
- [ ] Complete `meta bounded(...)` proof propagation across imported, generic,
      dynamic, recursive, and native call boundaries; the current source-level
      checker covers fixed control-flow, direct calls, fixed buffers, known
      local scalar sizes, and declared I/O effects.

## Planned metadata extension backlog

The following ideas are proposals, not accepted language features. Each item
must receive its own implementation work, tests, and focused guide/contract
documentation when work on that item begins. No guide file is created in
advance.

### Safety and semantic contracts

1. `meta must_use` — require callers to handle selected `Result`, `Option`, or
   status-producing values.
2. `meta effect(...)` — declare effects such as blocking, filesystem, entropy,
   allocation, or other compiler-recognized capabilities, with propagation to
   callers.
3. `meta realtime` — enforce the approved no-blocking, no-unbounded-work,
   no-floating-point, and no-forbidden-I/O policy for realtime verbs.
4. `meta bounded(max_iterations: ...)` — declare and validate an explicit
   execution bound where static proof is available.
5. `meta deterministic` — reject undeclared clock, entropy, filesystem-order,
   and other nondeterministic dependencies.

### Platform and native contracts

1. `meta capability(...)` — declare typed, target-specific platform
   capabilities and reject unavailable capabilities before code generation.
2. `meta repr("C")` — provide a restricted, explicitly checked ABI/layout
   contract for native interoperability.
3. `meta volatile` — define explicit hardware-observable access semantics only
   after the native memory-access contract is specified.
4. `meta zero_on_drop` — guarantee secret-material clearing across ownership
   moves and every cleanup path.
5. `meta stack_limit(...)` — enforce a bounded stack contract once the compiler
    has a sound interprocedural stack-size model.

### Ergonomics and compatibility

 1. `meta derive(...)` — generate only explicitly requested, documented,
    ownership-safe implementations such as equality or default construction.
 2. `meta deprecated(...)` — provide migration diagnostics without silently
    changing behavior.
 3. `meta pre(...)` and `meta post(...)` — add visible, typed contracts whose
    proof or runtime-check behavior is explicit rather than hidden.

### Backend optimization hints

 1. `meta unroll(...)`, `meta hot`, `meta cold`, and `meta simd(...)` — consider
    only after semantic contracts are stable; these hints must never change
    correctness, ownership, determinism, or safety guarantees.

The implementation order is intentional: semantic safety and effects precede
ABI/platform metadata; ergonomics follow the safety model; backend hints come
last. The phase remains open for adding and refining metadata proposals.

## Validation evidence

Recorded on 2026-10-29 at commit `7b5cd1f8`.

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

Known limitations remain explicit: this phase validates the current metadata
registry only. New attributes still require a separately reviewed design,
implementation, fixtures, and documentation. The local platform matrix uses
target-contract fixtures; native behavior on each release target is
additionally established by the corresponding CI jobs.

## Completion standard

The phase is complete only when the existing attributes have one consistent
documented contract, the relevant fixtures pass, diagnostics are stable, and
the implementation is ready for a separately reviewed proposal for any new
attribute.
