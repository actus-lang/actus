# Phase 18 Baseline Inventory

This inventory records the behavior that existed before the remaining strict
conformance work. It is the evidence boundary for Gate 18.0. The inventory is
descriptive: it does not treat a discovered fallback, compatibility path, or
documentation gap as acceptable strict behavior.

## Evidence sources

The inventory was checked against the following repository sources:

- `src/diagnostics/renderer/codes.rs` and `src/diagnostics/renderer/module.rs`;
- `src/parser/`, `src/semantic/`, `src/codegen/`, and `src/runtime/`;
- `src/cli/`, `src/configuration/`, and `src/modules/`;
- `library/std/src/` and `tests/library/`;
- `scripts/check_source_limits.sh`;
- `docs/language/diagnostics.md` and `docs/decisions/ADR-0041-strict-actus-conformance.md`.

The repeatable inventory commands are:

```sh
rg -o 'E[0-9]{4}' src tests docs | sort -u
find library/std/src -type f -name '*.act' | sort
find library/std/src -type f -name '*.act' -print0 | sort -z | xargs -0 wc -l
scripts/check_source_limits.sh
```

## Diagnostic catalog

`docs/language/diagnostics.md` is the public catalog and assigns these stable
ranges:

| Range | Current responsibility |
| --- | --- |
| `E0001`–`E0002` | Lexer failures |
| `E0003`–`E0004` | Parser token and end-of-input failures |
| `E0005`–`E0009` | Parser declarations, keywords, and metadata |
| `E1001`–`E1028` | Bindings, ownership, calls, types, and returns |
| `E1029`–`E1033` | Struct declarations and initialization |
| `E1034`–`E1035` | Struct mutation and field borrowing |
| `E1036`–`E1038` | Method lookup and receiver validation |
| `E1040`–`E1079` | Enums, roles, generics, packs, and arenas |
| `E1100`–`E1107` | Module paths, facades, sibling declarations, and module I/O |
| `E1800`–`E1809` | Strict configuration and policy |
| `E1810`–`E1819` | Strict frontend validation |
| `E1820`–`E1829` | Strict semantic validation |
| `E1830`–`E1839` | Strict architecture validation |
| `E1840`–`E1849` | Strict documentation validation |
| `E1850`–`E1859` | Strict source-limit validation |
| `E1860`–`E1899` | Strict execution and reserved expansion |

The catalog and renderer are separate from presentation. The terminal, JSON,
colored, and LSP renderers consume the same diagnostic model. The remaining
Gate 18.1 task must prove that selecting a renderer cannot change validation,
ordering, or exit status.

Known implementation-specific entries that require continued catalog review
include malformed generic-name handling (`E1080`), duplicate packed-layout
handling (`E1081`), and strict empty-enum validation (`E1810`). The next
catalog pass must verify that each such entry has one documented meaning, one
renderer mapping, and an accepted or rejected fixture.

## Recovery, fallback, and placeholder inventory

The following paths were found and classified for the next strictness gates:

| Location | Existing behavior | Phase 18 classification |
| --- | --- | --- |
| `src/codegen/case.rs` | Control-flow branches carry explicit fallback block values and unresolved inferred results are represented as `Void` in the current lowering contract. | The explicit control-flow value is supported; unresolved inference is forbidden in strict mode. Requires complete-AST/codegen evidence. |
| `src/codegen/expressions.rs` | Unresolved expression inference has a `Void` fallback in a lowering helper. | Forbidden in strict mode until replaced by an explicit error or proven non-codegen path. |
| `src/codegen/layout.rs` | Unknown role/width matches contain default branches. | Forbidden placeholder in strict mode; each branch needs an exhaustive domain proof or typed lowering error. |
| `src/codegen/enum_layout.rs` | Layout selection contains default branches for unsupported layout states. | Forbidden placeholder in strict mode; requires enum-layout fixtures and explicit errors. |
| `src/semantic/analyzer/mod.rs` | `Perform`, sibling `open`, and `import` declarations are intentionally non-codegen contracts. | Supported non-codegen meaning; must remain explicit and documented. |
| `src/parser/` | `unreachable!` is used after token-kind checks establish an internal parser invariant. | Supported internal invariant, not source recovery; retain only where mechanically established and tested. |
| `src/semantic/` | `unwrap_or("unknown")` and similar values are used to format incomplete diagnostic context. | Supported diagnostic-only fallback; it must never create an accepted semantic type or executable path. |
| `src/runtime/` | `try_from(...).unwrap_or(-1)` maps unrepresentable results to the documented C-ABI failure status. | Supported ABI contract, not a semantic default; preserve only with native boundary tests and explicit status documentation. |

This table separates supported language meaning from unresolved compiler state.
Gate 18.3 must close the code-generation audit before strict mode can claim
that every accepted syntax form is either executable or explicitly non-codegen.

## CLI, statuses, filtering, and targets

The command dispatcher currently exposes `new`, `init`, `check`, `parse`,
`build`, `run`, `watch`, `test`, `fmt`, `lsp`, and `publish`. Unknown commands,
missing command arguments, extra arguments, and malformed command options use
status `2`. Source, configuration, module, semantic, build, and runtime
failures use status `1`; successful commands use status `0`.

Strict mode is currently accepted by `check`, `build`, and `test`. It is parsed
once, rejects duplicate `--strict`, uses strict configuration policy, and is
shown in successful command output. `parse`, `run`, `watch`, `fmt`, `lsp`, and
`publish` do not currently participate in the strict conformance contract.
That boundary must be documented in the eventual compatibility matrix rather
than inferred from command names.

Build and test input discovery can walk to a parent `Actus.toml`. Strict
configuration rejects legacy `Arca.toml` manifests and validates the lockfile
policy. Module resolution requires the directory facade named after the
module, rejects ambiguous file/directory roots, sorts sibling declarations,
and reports unknown or duplicate module declarations through module-specific
diagnostics.

Target metadata is applied before semantic validation. Unknown target selectors
are parser failures, and non-matching declarations are filtered before the
remaining program is analyzed. The current implementation has regression
coverage for target filtering and symbol references; malformed or conflicting
target declarations remain a later Gate 18.9 task.

The test runner discovers `meta test` verbs, reports discovered and filtered
counts, passes null stdin to native test processes, and terminates a process
after its bounded timeout. A non-zero process, timeout, or failure contributes
to a non-zero test result. Lockfile freshness, artifact determinism, and the
full strict compatibility matrix remain unclosed.

## Repository integrity checks

The current source-limit script scans only `src/**/*.rs`. It enforces the
500-line file hard limit and 60-line function hard limit, but it does not yet
enforce the preferred 300-line size, the 400-line warning threshold, Actus
`.act` files, or strict diagnostic reporting. The largest current Rust files
remain below the hard limit, while several are at or above the warning range;
decomposition is therefore still required before strict source-limit closure.

Module discovery is deterministic at the resolver boundary: module segments
must be valid identifiers, a directory module requires its `<module>.act`
facade, a file root and directory root together are rejected as ambiguous,
and sibling `.act` paths are sorted after discovery. I/O, missing-facade,
invalid-path, and ambiguity failures remain explicit `ModuleResolutionError`
variants.

Lockfile validation parses `Actus.lock`, checks version and deterministic
package ordering, regenerates the dependency graph from `Actus.toml`, and
compares locked packages with the generated package set. Normal configuration
can synchronize a missing lockfile; strict configuration requires the existing
lockfile policy and rejects stale content. Source-content checksums and
artifact reproducibility still require dedicated strict fixtures.

The test runner recursively discovers `.act` files below `tests/` and the
configured source root, excludes `fixtures` and `snapshots`, sorts files and
test names, applies target filtering, and reports filtered counts. It sends
null stdin to each native test process, applies a 30-second timeout, removes
temporary object and executable files, and returns failure for non-zero exits
or runner errors. Signal-specific reporting and cross-platform cleanup remain
open integrity work.

No existing fallback is classified as deprecated in this baseline. Every
accepted compatibility path is either supported with a documented contract or
is explicitly forbidden in strict mode as shown above; this prevents an
unreviewed fallback from silently becoming a third behavior category.

## Gate 18.0 conformance matrix

The following is the target contract for the first strict-mode matrix. It
defines expected results; it does not claim that every row is implemented yet.

| Command | Normal mode | Strict mode |
| --- | --- | --- |
| `actus check` | Parse, resolve modules, apply target filtering, and run semantic validation; success is status `0`, source/configuration failure is `1`. | Perform the same validation with strict configuration and no executable artifact; unresolved policy state is a failure, success is status `0`. |
| `actus build` | Complete validation followed by code generation and linking; source/configuration/build failure is `1`. | Complete strict validation before code generation; any error, promoted warning, unresolved state, or stale lockfile prevents a usable artifact and returns `1`. |
| `actus test` | Discover, compile, and run meta tests; a failed, timed-out, signaled, or non-zero test returns `1`. | Apply strict configuration and semantic preflight before discovery/execution; the same runtime failures return `1`, with deterministic counts and filtering. |

Across both modes, malformed command arguments return `2`; lexer, parser,
module, semantic, ownership, and configuration violations return `1`; and a
successful command returns `0`. Diagnostics are errors at the command
boundary even when their renderer is text, color, JSON, or LSP. Renderer choice
may change presentation only, never acceptance, ordering, or exit status.

This matrix is intentionally limited to `check`, `build`, and `test`, the
commands that currently accept `--strict`. The behavior of `run`, `watch`,
`fmt`, `parse`, `lsp`, and `publish` must not be inferred from this matrix.

## Stable exit-status categories

The CLI keeps a small numeric status surface and exposes the detailed category
through the diagnostic or test-result output:

| Category | Status | Meaning |
| --- | ---: | --- |
| Success | `0` | The requested command completed its declared contract. |
| Usage | `2` | The command, option, argument count, or option value is invalid. |
| Source | `1` | Lexing, parsing, module resolution, semantic, ownership, or code-generation validation failed. |
| Configuration | `1` | Manifest, dependency, target, profile, or lockfile policy failed. |
| Test | `1` | A meta test failed, timed out, received an invalid process result, or could not be executed. |
| Runtime | `1` | A built program or runtime bridge returned a failure result. |
| Infrastructure | `1` | Required file, linker, process, or host operation failed. |

The categories intentionally share status `1` so shell callers retain the
existing contract while diagnostics remain precise. A future numeric change
would require a documented compatibility decision and command-level fixtures;
new source, configuration, test, runtime, and infrastructure failures must not
silently become success or usage status.

## Minimum gate evidence

A roadmap checkbox may be marked complete only when all applicable evidence is
available:

1. the implementation or documented contract names its responsible module;
2. at least one accepted and one rejected fixture cover the rule, unless the
   task is purely an inventory or documentation task;
3. the command output and exit status are captured for both relevant outcomes;
4. artifact creation, cleanup, and lockfile effects are checked when the task
   can produce them;
5. repeated execution produces the same diagnostics, ordering, counts, and
   status; and
6. the required repository quality commands pass.

Cross-platform or ABI behavior additionally requires native execution evidence
on every applicable target. A passing compilation without the corresponding
execution or rejection evidence is not sufficient.

## Compatibility-break review record

The following intentional differences between normal and strict behavior are
recorded before implementation expands the strict boundary:

| Existing compatibility path | Strict policy | Reason and follow-up |
| --- | --- | --- |
| Normal configuration can accept legacy `Arca.toml` manifests with compatibility handling. | Reject the legacy root and dependency manifest with stable strict diagnostics. | Prevents ambiguous project identity; covered by strict configuration fixtures. |
| Normal configuration can synchronize a missing lockfile. | Require the lockfile policy and reject stale or absent required content. | Makes dependency selection reproducible; close with lockfile command fixtures. |
| The source-limit script currently checks only Rust hard limits. | Extend strict validation to preferred/warning thresholds and Actus source boundaries. | Prevents standard-library and documentation drift; close in Gate 18.8/18.10. |
| Strict mode currently applies only to `check`, `build`, and `test`. | Do not imply strict guarantees for `run`, `watch`, `fmt`, `parse`, `lsp`, or `publish`. | Avoids an undocumented partial contract; expand only through a reviewed roadmap change. |
| Normal test execution can discover and run tests without strict semantic preflight. | Require strict configuration and semantic preflight before test execution. | Prevents executable tests from hiding unresolved source state; close with runner fixtures. |

Each row is an approved compatibility boundary for this phase, not a waiver of
the strict contract. Any additional compatibility break must add a row here,
an ADR/roadmap reference, and positive and negative command evidence before its
checkbox is marked complete.

## Standard-library inventory

`library/std/src/` currently contains 29 Actus files and 1,941 lines:

| Facade/module | Files and responsibility |
| --- | --- |
| `io/io.act` | Public facade for stdin, stdout, stderr, errors, readers, writers, cursor, copying, and buffering. |
| `fs/fs.act` | Public facade for files, metadata, operations, options, and seeking. |
| `path/path.act` | Public facade for path types, storage, errors, components, platform parsers, predicates, normalization, and builders. |
| `lib.act` | Standard-library root facade. |

The current syntactic count is 195 public-looking declarations in the `io`,
`fs`, and `path` trees and 290 `"""`/`///` documentation markers. These are
inventory counts, not a coverage result: one block may document a declaration,
an ABI bridge, or a grouping comment. Gate 18.10 must perform declaration-level
matching and require documentation for purpose, ownership roles, return/error
contracts, side effects, and ABI relationships.

The public standard-library boundary is Actus code. Native behavior is reached
through typed `unsafe extern "C"` declarations and runtime implementations;
the inventory therefore records both the facade declaration and the native
test evidence as separate obligations. Standard-library fixtures belong under
`tests/library/`; Rust tests must not replace Actus contract fixtures.

## Baseline decision

The first four Gate 18.0 inventory tasks are evidenced by this document. The
remaining inventory tasks are intentionally open: source-limit enforcement,
module discovery, lockfile validation, test-runner validation, and the complete
fallback classification require implementation or additional fixtures. No
later gate is considered complete merely because a related implementation
already exists.
