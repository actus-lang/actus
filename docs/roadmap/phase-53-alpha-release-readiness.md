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
- R1: source-level validation accepts the program after normalization;
  generic specialization (`specialize_program`) creates distinct instances
  for `Result[Int, ParseError]` and `Result[Int, config::ParseError]`, so
  orchestration fails. PR A reports this as an internal compiler error instead
  of a misleading entry-file E1027.
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

## A0 evidence

- `18d700bb` implements ADR-0107 SourceMap provenance: compilation-session
  offsets start at 1, source identities are deterministic and canonicalized,
  and diagnostics localize primary and related spans before rendering.
- `c3ed40f1` adds check/build regression coverage for module lexical, parser,
  and cross-file duplicate diagnostics. SourceMap unit tests cover EOF gaps,
  canonical relative/absolute paths, and related-location localization.
- `cargo fmt --check`, `cargo check --all-features`, Clippy for all-features,
  Cranelift-only, and LLVM-only configurations, source limits, architecture,
  documentation, secrets, and `cargo test --all-targets --all-features` pass.
- The test list is `1,994`: the base list was `1,988`, with six new A0 tests
  and no removed tests. Existing snapshots and tests were not modified.
- On the neutral R1 reproduction, both base and A0 `check --strict` pass;
  base build reports the entry path at `3:17`, while A0 reports the owning
  module path `src/app/start.act` at `3:5` with E1027.
- Regression fixed in `2392c401`: duplicate-declaration identity no longer
  depends on source spans (std test suite E1110).

## PR A evidence

- The R1 test is `reexported_type_identity_passes_validation_and_fails_in_specialization`:
  check passes; build fails with the expected internal compiler error and E1027.
- The distinct-enum negative test passes with E1027 attributed to
  `src/app/start.act`.
- Reproduction matrix, base `964a83fb` versus PR A:

  | Reproduction | Base | PR A |
  |---|---|---|
  | `repro-r1` | check PASS; build E1027 at `3:6` | check PASS; build ICE containing E1027 |
  | `repro-r1-struct` | check PASS; build E1027 at `3:6` | check PASS; build ICE containing E1027 |
  | `q1-one-open-clean` | check PASS; build E1069 at `5:1` | check/build E1069 at `3:54` |
  | `repro-r2-min` | check/build unavailable: E1803 compiler-owned stdlib | check/build E1052 in an imported module |

- Private downstream application: base check passes and build reports E1027;
  the snapshot build reports E1052 for `N`. PR A check, build, and snapshot
  build report E1052 for a re-exported const-generic struct; base `test --strict --list`
  produced a list, while the PR A invocation did not complete in this
  environment.
- The measured PR A test list is `2,002`; module diagnostics are `8/8`, the
  standard-library suite is `112/112`, serialization is `10/10`, and the full
  all-targets suite passes. Formatting, all three Clippy configurations,
  source limits, architecture, documentation, and secrets checks pass.
- Release timing (base `964a83fb` | PR A | overhead): private-app check
  `3,475 ms | 4,205 ms | +21.0%`; phase23 non-strict build
  `8 ms | 9 ms | +12.5%`; std suite `2,213 ms | 2,636 ms | +19.1%`.
  Check and build now share validation; the cost is approximately 20% on the
  largest program. A parse cache (28,616 scans/parses of the same files per
  check) and grouped test binaries (PR T) are the planned reductions.
- The release examples matrix remained `33/33 identical` for LLVM and
  Cranelift against the Phase 53 audit baseline. The generated-serialization
  validation test passes with normalization and fails with E1069 when
  normalization is temporarily removed.
- PR S replaces the former PR P parse-cache item and precedes PR B. Release
  evidence against PR A: private check `2379 -> 1542 ms (-35.2%)`; std suite
  `2363 -> 2365 ms (+0.1%)`; phase23 build, interleaved, 100 pairs
  `10.97 +/- 5.01 -> 11.11 +/- 4.90 ms`, pairwise `B-A 0.14 +/- 4.57 ms`;
  VmHWM private `130348-131112 -> 135980-136828 KiB`, std
  `691248-698020 -> 690384-693344 KiB`; parse `18372/80`, `18754/151`.
- PR B release evidence against PR S, interleaved, 3 warm-up and 20 pairs:
  private check `2365.8 -> 2513.7 ms (+6.25%)`; std suite
  `2170.6 -> 2289.4 ms (+5.47%)`; phase23 build `15.16 -> 16.19 ms (+6.79%)`;
  VmHWM private `166 -> 257 MiB`, std `684 -> 816 MiB`. The full suite passes
  (82 targets, 2,039 tests), std `112/112`, the examples matrix is `33/33`
  against PR S and between LLVM and Cranelift, `nm` symbol names match PR S,
  and repeated builds produce bit-identical objects on both backends.

1. **A0 — SourceMap and source provenance** (`ADR-0107`): give compilation
   units deterministic source identity and report unit diagnostics against
   their owning file.
2. **A — Unified compilation-plan validation**: make check, build, run, and
   test use the same plan validation without changing buildable behavior.
   Normalization, including generated serialization verbs, is part of
   validation; generic specialization is not.
3. **S — Compilation session and memoized frontend queries** (`ADR-0109`):
   one session owns source provenance and memoized parse results for each CLI
   operation and for the complete test run. Plan construction parses only
   through the session; a changed source is a new revision. The former PR P
   parse-cache item is replaced by S. PR B adds the declaration registry and
   annotated units as further session queries.
4. **B — Canonical declaration identity** (`ADR-0108`): generic instances and
   all downstream consumers use `package + module path + declaration name +
   ordered arguments`, independent of import aliases and re-export spellings.
   The timing gate is release-versus-release, with PR B overhead no more than
   10% relative to its parent PR S. The R1 test must check, build, and run
   successfully.
5. **B2 — Generic verb canonical identity**: generic verb specialization still
   contains spelling-based handling in `src/codegen/generic/verb_body.rs`
   (`specialize_call`, `callee.contains('[')`), and related generic-verb name
   matching in `src/codegen/generic/verbs.rs`,
   `src/cli/build/emission/llvm.rs`, and `src/cli/build/emission/objects.rs`.
   This is a latent defect: two modules with same-named generic verbs would be
   matched by short name. It is not observable today, because E1110 rejects
   same-named declarations across the modules of one program; PR B2 removes it
   before any change to the E1110 rule.
6. **B3 — Semantic type representation**: replace the remaining semantic
   string representation with `expression_type_name -> Option<TypeName>`,
   consolidate the `canonical_type_name` variants into one display function,
   and use identity for semantic comparisons, including checks currently
   expressed as `expected.name == "Result"`. PR B3 empties the explicit
   `pr_b3_allowlisted` entries in `tests/architecture_conformance.rs`. The
   spelling-based comparison is not observable today for the same E1110 reason.
7. **C — Hierarchical module scope and R2** (`ADR-0110`): an import written
   inside a unit that names one of the unit's own directories (for example
   `import pkg::runtime` in `pkg/runtime/training/helper.act`) loaded that
   directory again as a second, incomplete unit, so a const-generic struct of
   a sibling directory was missing and resolution failed with `E1052` (the
   minimal R2 reproduction). C binds such imports to the loaded unit and
   enforces ADR-0018's module boundaries hierarchically: a directory module
   sees its own and its ancestors' declarations, the exports of the children
   it opens, and its own imports; sibling directories need an import and
   imports are not inherited. Out-of-scope references report `E1119`.
   Related defects found on the same path are fixed in C: a `case` block
   branch that leaves through `return` no longer types the `case`, a
   qualified call to another unit's verb is typed in assignments and lowered
   when generic, and named values compare by declaration identity. PR B
   already rejects private type arguments of an imported unit at `check`
   (`E1109`, `tests/declaration_identity.rs`); the earlier "second
   reproduction" was a malformed fixture and is withdrawn.
   An import of another unit's opened child directory (for example
   `import pkg::store::cache` in a different unit) binds to the owning unit
   with the child's exports only, instead of loading the directory as a
   second unit.
8. **U — Unit-scoped facade opening** (`ADR-0111`): a facade may write
   `open unit x;` to open a sibling file or child directory for its unit
   only. Unit-scoped names follow ADR-0110 inside the unit and are not
   exports of the unit (`E1109` from another unit, `E1119` through a child
   import). Downstream applications that published internal helpers to
   satisfy ADR-0110 can keep them inside the unit. New diagnostic `E1137`
   (one name opened both ways). Narrowing the symbol linkage of non-public
   verbs remains a separate decision.
9. **T — Grouped test binaries** (new ADR): `actus test` builds one binary per
   module (all tests of the module's `tests/` files), runs each test in its own
   process selected by name (isolation, timeouts and expected faults unchanged),
   and validates the compilation plan once per group. A group compile error
   fails every test in that group with one diagnostic.
10. **D — Diagnostic rendering**: replace backend raw `Debug` errors with the
   stable diagnostic contract and an explicit internal-error boundary.
11. **E — Formatter layout rule and `actus new`** (new ADR): verb bodies are
   always multi-line; call arguments, verb parameters, struct fields,
   struct-literal fields and enum variants are placed one per line when there
   are two or more, and stay on one line when there is one. Type arguments,
   generic parameters and array literals keep the width rule. The formatter
   must never emit a line over the configured width (today two std call sites
   are joined to ~103 columns). Reformat library/std, examples and fixtures,
   make `actus new` output pass `actus fmt --check`, and fix the three std files
   that are not fmt-clean (`fs/directory.act`,
   `wire/fragment/reassembly.act`, `volatile/volatile.act`).
12. **F — Documentation reconciliation**: correct README examples, alpha guide
   commands, ADR statuses, root-manifest documentation, and add the E1800
   `actus lock` hint.
13. **F2 — Neutral fixtures and documentation**: replace downstream-application
   names and private implementation terms in docs, tests, examples, and std
   comments with neutral names without changing test behavior.
14. **L — LSP workspace session** (`ADR-0109`): the language server keeps one
   compilation session per workspace, routes overlay parsing through it, and
   invalidates memoized results by document version.
15. **G — Version and release note**: reconcile the compiler version, alpha tag,
   release notes, and final release evidence.

## Evidence gates

Each PR records formatter, clippy, source-limit, architecture, documentation,
secret, test-list, and full-test evidence. Example check/build/run parity is
required; changes outside the stated diagnostic or IR provenance effects stop
the phase for review.

