# Phase 53: Alpha Release Readiness

## Status: In progress

Phase 53 closes the remaining alpha-readiness contracts exposed by the
pre-release audit. It preserves the compiler's existing language and ABI
contracts while making diagnostics, documentation, and release evidence
consistent.

## Audit findings

- Root cause: `check` analyzes only the caller, while build analyzes the
  caller, object units, and the performance-context program. Therefore check
  passes while build fails.
- Provenance: a unit diagnostic is attributed to the entry file (`R1`:
  `main.act`), although the source is line 3 of `src/app/start.act`; the exact
  column is established by the A0 provenance test.
- Span-keyed caches (`src/semantic/generics/substitution.rs:75`,
  `src/semantic/generics/cache.rs:34`, and `src/semantic/analyzer/state.rs:84`)
  and synthetic names depend on file-local offsets, so cross-file collisions
  are possible; this is a possible R2 cause to verify after A0.
- R1: the same enum is available through two re-export paths
  (`config::ParseError` versus `app::ParseError`) and build
  resolves them as two types (E1027). The direct struct reproduction without
  `Result` does not reproduce the failure.
- R2: a re-exported const-generic struct with a `meta limitless` performance
  context reaches build as raw `UnknownTypeParameter`; the minimal
  reproduction reports E1052 for `N`.
- Raw `Debug`: `src/codegen/shared_lowering/orchestration.rs:73,275`,
  `src/cli/build/emission/shared.rs:66,75,79,158,164`,
  `src/cli/build/emission/objects.rs:66`,
  `src/codegen/cranelift/emission/output.rs:84,93,96,227`,
  `src/codegen/cranelift/contract/functions.rs:58`,
  `src/codegen/cranelift/function_definition/definition.rs:69`,
  `src/codegen/cranelift/generic/enum_layout.rs:25`,
  `src/codegen/cranelift/generic/struct_layout.rs:25`,
  `src/codegen/generic/definitions.rs:282`,
  `src/codegen/generic/verbs.rs:41,147,256,318,357`, and
  `src/cli/test_runner/collection.rs:189`.
- `actus new`: generated `src/main.act` fails `fmt --check` because its verb
  is on one line.
- README: `cargo run --` commands omit `--bin actus`; they fail because the
  workspace has two binaries.
- ADR-0106 remains Proposed although Phase 52 is complete. ADR-0047 is
  Implemented, but not Accepted.
- Root manifest: `entry = "main"`, but `src/main.act` does not exist. Root
  check/build/watch fail while test works; the alpha documentation describes
  the root package. Library-only packages are deferred until after alpha and
  require an ADR.
- E1800 `LockfileOutOfDate`: every std change changes `runtime_checksum`, so
  project lockfiles become stale. The diagnostic lacks a direct `actus lock`
  hint.
- Private downstream application: tests take 33–58 seconds each when run
  serially and time out under parallel execution; test performance is deferred
  until after alpha. `--list` is empty for the private downstream application
  but works for std; this remains an investigation item.

## Audit evidence

- The release dry-run passed in the pinned `manylinux_2_28` environment,
  including the verifier and QEMU.
- The private downstream application produced identical results with the
  pre-52 and post-52 compilers.
- 33 examples produced identical LLVM and Cranelift results.
- The standard-library suite passed 112 tests.

## Reproductions

The reproduction consists of `Actus.toml`, `Actus.lock`, and seven Actus
sources.

`Actus.toml`

```toml
[package]
name = "repro-r1"
version = "0.1.0"
edition = "alpha"
entry = "main"
```

`Actus.lock`

```toml
lockfile_version = 3
runtime = "core"
package = []
```

`src/main.act`

```text
import app;
verb main() -> Int {
    return app::start();
}
```

`src/app/app.act`

```text
open start;
```

`src/interface/interface.act`

```text
open config;
open parser;
```

`src/interface/parser.act`

```text
import interface::config;
open verb parse() -> Result[Int, config::ParseError] {
    return Result[Int, config::ParseError].Ok(0);
}
```

`src/interface/config/config.act`

```text
open contract;
```

`src/interface/config/contract.act`

```text
open enum ParseError { Invalid, }
```

`src/app/start.act`

```text
import interface;
open verb start() -> Int {
    erg result: Result[Int, interface::ParseError] = interface::parse();
    return 0;
}
```

## Ordered PRs

1. **A0 — SourceMap and source provenance** (`ADR-0107`): give compilation
   units deterministic source identity and report unit diagnostics against
   their owning file.
2. **A — Unified compilation-plan validation**: make check, build, run, and
   test use the same plan validation without changing buildable behavior.
3. **B — R1 canonical type identity across re-exports**: one declaration
   reached through different `open` re-export paths must resolve to one type
   in every unit.
4. **C — R2 const-generic resolution**: resolve the re-exported
   const-generic struct type-parameter path exposed by the minimal R2
   reproduction.
5. **D — Diagnostic rendering**: replace backend raw `Debug` errors with the
   stable diagnostic contract and an explicit internal-error boundary.
6. **E — `actus new` formatting**: make generated projects pass the formatter
   contract and add regression evidence.
7. **F — Documentation reconciliation**: correct README examples, alpha guide
   commands, ADR statuses, root-manifest documentation, and add the E1800
   `actus lock` hint.
8. **F2 — Neutral fixtures and documentation**: replace downstream-application
   names and private implementation terms in docs, tests, examples, and std
   comments with neutral names without changing test behavior.
9. **G — Version and release note**: reconcile the compiler version, alpha tag,
   release notes, and final release evidence.

## Evidence gates

Each PR records formatter, clippy, source-limit, architecture, documentation,
secret, test-list, and full-test evidence. Example check/build/run parity is
required; changes outside the stated diagnostic or IR provenance effects stop
the phase for review.

