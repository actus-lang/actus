# Phase 19 Gate 19.0: Baseline and Contract Inventory

This report records the Gate 19.0 baseline for
[ADR-0045](../decisions/ADR-0045-alpha-core-and-application-readiness.md).
It is an inventory, not an implementation claim. A row is marked complete only
when the repository contains the referenced contract or evidence.

## Baseline identity

| Item | Current baseline |
| --- | --- |
| Compiler package | `actus` `0.1.0` |
| Actus package edition | `alpha` |
| Project entry | `Actus.toml`, entry `main` |
| Standard-library package | `std` `0.1.0`, source root `src`, entry `lib` |
| Compiler implementation edition | Rust `2024` |
| Language pipeline | lexer -> parser -> AST -> semantic -> codegen |
| Ownership roles | `erg`, `abs`, `dat`, `ins` |
| Baseline source revision | `f59a03d` (`main`, ADR-0044 merge) |
| Working branch | `feat/adr-0045-alpha-core-application-readiness` |
| Supported CI hosts | Linux, macOS, Windows |
| Primary native backend | Rust-native Cranelift object/executable backend |
| Public binary boundary | Explicit runtime/C-ABI bridges; internal Actus ABI remains Alpha-scoped |

The Alpha compiler invocation is the `actus` CLI. The accepted validation path
uses `actus check`, `actus build`, `actus run`, `actus test`, `actus fmt`,
`actus watch`, and `actus lock` according to each command's documented
arguments. `--strict` is currently accepted by the strict frontend commands
that expose it; `actus run` currently uses the normal run option surface and
does not accept `--strict`. This distinction remains an implementation fact to
be resolved or documented during Gates 19.3 and 19.5.

## Remaining Phase 16 Gate 6 inventory

| Existing item | Phase 19 assignment | Current state |
| --- | --- | --- |
| Console application with `std::io` | Gate 19.2 | Scheduled; no accepted real application yet |
| File utility with `std::fs`/`std::path` | Gate 19.2 | Scheduled; library fixtures exist, application evidence is pending |
| Build and run both applications | Gate 19.2 | Scheduled |
| stdout, stderr, and exit codes | Gate 19.2 | Runtime primitives exist; application evidence is pending |
| `actus check` without code generation | Gate 19.3 | Implemented in CLI; application workflow coverage pending |
| `actus test` library/application workflow | Gate 19.3 | Library meta-test workflow exists; application matrix is pending |
| `actus fmt` and `fmt --check` | Gate 19.3 | Existing CLI tests exist; Alpha workflow acceptance is pending |
| `actus watch --once` and continuous watch | Gate 19.3 | Existing command/tests exist; full application workflow evidence is pending |
| Facade imports and standard-library exports | Gate 19.2/19.3 | Implemented fixtures exist; external application acceptance is pending |
| Debug/release profiles | Gate 19.3 | Configuration and CLI support exist; application artifact evidence is pending |
| Deterministic artifacts and lockfile behavior | Gate 19.3/19.6 | Lockfile and deterministic compiler tests exist; release report is pending |
| Beginner-to-running-program documentation | Gate 19.5 | Partial documentation exists; complete verified tutorial is pending |
| Full quality and portability matrix | Gate 19.4/19.6 | CI is green for the current branch; Phase 19 application matrix is pending |
| Deferred host services and bare-metal substitutions | Gate 19.4/19.5 | Several ADRs record boundaries; consolidated Alpha inventory is pending |

No Phase 16 item is silently discarded. Items remain unchecked in the Phase 16
roadmap until their original acceptance criteria are met; this table assigns
their implementation and evidence owner in Phase 19.

## Remaining Phase 11 inventory

All remaining Phase 11 items are assigned to Gate 19.1 unless noted otherwise:

| Unfinished contract | Assignment | Required outcome |
| --- | --- | --- |
| Field visibility and access | Gate 19.1 | Explicit module/field visibility rules and diagnostics |
| `erg`/`abs`/`dat` field ownership | Gate 19.1 | State transitions and accepted/rejected fixtures |
| Copy, move, and assignment behavior | Gate 19.1 | Whole and partial value rules with cleanup evidence |
| Self-referential and escaping borrow fields | Gate 19.1 | Explicit Alpha rejection or a documented lifetime contract |
| Packed/external struct representation | Gate 19.1 | Representation policy, layout tests, and unsupported cases |
| Native struct layouts and declarations | Gate 19.1 | Backend output contract consumed from validated semantic layout |
| Public struct C ABI | Gate 19.1 | Stable boundary or explicit Alpha deferral |
| Complete struct documentation | Gate 19.1/19.5 | Language reference and examples match implementation |
| Struct architecture records | Gate 19.1 | ADR coverage for ownership, layout, and ABI decisions |
| Struct migration rules | Gate 19.5 | Compatibility classification for Alpha changes |
| Manifesto/spec/implementation synchronization | Gate 19.5 | Cross-document consistency check |

## Public contract inventory

### Compiler and semantic contracts

- Lexer tokens and source spans are defined under `src/lexer/`.
- Grammar and expression precedence are defined under `src/parser/` and
  `src/ast/`.
- Ownership, borrowing, cleanup, type validation, operators, arrays, casts,
  packs, and target filtering are semantic responsibilities under `src/semantic/`.
- Cranelift lowering and ABI selection are code-generation responsibilities
  under `src/codegen/`.
- Diagnostics are modeled independently from terminal rendering under
  `src/diagnostics/`.

### Runtime and standard-library contracts

- Runtime bridges are isolated under `src/runtime/` and expose documented
  status/ownership boundaries.
- The canonical standard-library facade is `library/std/src/lib.act`.
- `std::io` is split into reader, writer, cursor, buffering, copying, input,
  output, and error siblings under `library/std/src/io/`.
- `std::fs` is split into file, metadata, options, seek, operations, and error
  siblings under `library/std/src/fs/`.
- `std::path` is split into representation, storage, components, parsers,
  predicates, normalization, builders, and error siblings under
  `library/std/src/path/`.
- Public fallible filesystem and I/O operations return typed `Result` values;
  raw host/C statuses remain private to runtime bridges.

### CLI and package contracts

- CLI commands are declared and validated under `src/cli/`.
- Project configuration is described by `Actus.toml`.
- Dependency state is represented by `Actus.lock` and validated by the lock
  command and configuration layer.
- Build profiles, targets, linkers, entry contracts, and artifact paths are
  owned by `src/configuration/` and `src/target*` modules.
- The CLI currently exposes project creation, checking, parsing, building,
  running, watching, testing, conformance, formatting, LSP, lockfile, and
  publishing commands.

### Existing evidence boundaries

- Compiler unit, semantic, codegen, CLI, runtime, and standard-library tests
  provide broad feature evidence.
- `examples/operator_surface.act` provides executable evidence for the current
  ADR-0042 through ADR-0044 operator and storage surface.
- `tests/library/` contains Actus meta-test fixtures for standard-library
  contracts; it is not a substitute for real application acceptance.
- CI currently verifies Linux, macOS, and Windows Rust checks, coverage, and
  dependency/license policy for the compiler repository.
- Tree-sitter and VS Code are separate repositories and their synchronized
  commits are recorded in ADR-0044 Gate 7 evidence.

## Explicit deferrals and future ownership

The following are not Alpha Gate 19.0 implementation claims:

- Phase 17 extension product features remain deferred.
- General lifetime relationships, escaping views, and advanced reborrow chains
  remain deferred until a separate ownership decision accepts them.
- Self-hosting compiler stages remain a future direction.
- Concurrency, task transfer, and thread-safety semantics remain outside this
  baseline unless a later ADR explicitly schedules them.
- Remote registry services and web authentication remain outside the local
  Alpha compiler acceptance scope.

Each deferral has an owner in ADR-0045 or an existing architectural record; no
deferred item is treated as an implicit Alpha guarantee.

## Documentation consistency findings

The following consistency actions are recorded for later gates:

1. `ROADMAP.md` previously listed Phase 18 as the latest completed phase and
   did not list Phase 19; it must now list Phase 19 as the active unchecked
   Alpha readiness phase.
2. Phase 16 Gate 6 remains intentionally unchecked because its application and
   completion evidence is not yet present. Phase 19 assigns each item without
   falsely marking Phase 16 complete.
3. Phase 11 remains intentionally unchecked for unresolved struct contracts.
4. ADR-0044 is accepted and its compiler/LSP/Tree-sitter/VS Code synchronization
   evidence is current.
5. Existing Alpha documentation describes the restricted, non-escaping borrow
   model and unstable internal ABI; those claims remain compatible with this
   baseline and must be preserved during implementation.

## Gate 19.0 exit decision

Gate 19.0 is complete when this report, the Phase 19 roadmap assignments, and
the top-level roadmap identify the same remaining work. The next implementation
gate is Gate 19.1, where the unresolved struct semantics are converted into
explicit language and ABI contracts before application work depends on them.
