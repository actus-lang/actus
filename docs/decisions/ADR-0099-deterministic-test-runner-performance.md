# ADR-0099: Deterministic and isolated test-runner performance

- **Status:** Proposed
- **Date:** 2026-11-02
- **Scope:** Actus test discovery, compilation, native object reuse, process
  isolation, output ordering, and test-runner performance

## Context

`actus test` compiles and executes Actus tests through the native compiler
pipeline. A large suite can therefore repeat frontend analysis, native object
emission, linking, temporary-file setup, and process startup for every test.
The cost is especially visible when a test file imports the same modules for
many test verbs.

The runner must become faster without changing the language, ownership rules,
backend contracts, cleanup semantics, fault injection behavior, or the meaning
of an individual test. A passing test must still run against the same compiled
program contract that it would run under the unoptimized runner.

## Decision

Actus will improve test-runner performance in explicit, reviewable stages.
The runner will preserve isolation and deterministic reporting as the primary
contracts; performance optimizations may not infer safety from a successful
build or merge independent native programs without a checked compatibility
contract.

### 1. Deterministic parallel preparation

Test-file discovery and compilation-plan preparation may run in parallel using
bounded host parallelism. Each worker owns its resolver and compilation state;
workers must not share mutable semantic, code-generation, temporary-output, or
fault state.

Results are merged in discovery order and the final test list remains sorted by
the existing path/name ordering. Diagnostics, test numbering, filtering, and
the visible test order therefore remain stable across different worker counts.

The same bounded worker policy may execute ordinary selected tests concurrently.
Each test still gets its own native executable, temporary artifact directory,
working directory, timeout, and child process. Workers return structured test
results; only the coordinator prints them, in canonical test order. A test
that exits, traps, times out, or receives a signal therefore cannot terminate
or overwrite a sibling test process.

The default worker count is the host's available parallelism, capped by the
number of selected tests. `--test-threads N` may reduce that count for
constrained or intentionally serialized environments; zero and invalid values
are rejected. Parallel execution is not allowed to change test selection, test
numbering, failure classification, or output order.

### 2. Safe module-artifact reuse

When multiple tests from the same compilation unit import identical modules,
the runner may reuse their emitted module objects only after the cache key
proves all inputs that affect code generation are identical. The key must
include, at minimum:

- source and imported-module identities with content or compiler-recognized
  revision information;
- the complete compilation plan and generic-instance set;
- selected backend, target, ABI, feature configuration, and compiler version;
- strictness and native contract settings; and
- the object-emission format and relevant runtime artifact identity.

The generated test root/harness object is never reused merely because two test
verbs have the same signature. Root objects remain test-specific. Cached
objects are immutable and are written to test-owned output or linked through a
read-only path; cleanup of one test must not delete an object used by another.

### 3. Execution isolation

Ordinary tests may share compilation work, but each test retains its defined
runtime state and failure boundary. Tests that intentionally exercise process
exit, signals, traps, injected faults, or abnormal termination continue to run
in isolated subprocesses. The runner must not combine such tests into a shared
process or executable unless a later decision specifies equivalent isolation
and evidence.

### 4. Rust-style grouping is conditional, not implicit

Building one dispatcher executable per source file is an optional future
optimization. It is allowed only when every selected test in the group has a
compatible return contract, does not require a distinct native entry boundary,
does not rely on process-level isolation, and can preserve per-test cleanup,
fault behavior, output, and exit status. A grouped executable must report
results in the established deterministic order and must not turn one test's
panic, trap, or leaked state into another test's result.

Until those conditions are proven, the runner will use shared compilation
artifacts plus isolated test execution rather than silently grouping tests.

### 5. Failure and diagnostic contract

An optimization failure is not converted into a passing test. If an artifact
cannot be reused or a grouped execution contract cannot be proven, the runner
falls back to the existing isolated path and records the reason at diagnostic
verbosity levels. Compilation, linking, execution, timeout, signal, and exit
status failures retain their current classification and source context.

## Invariants

- Test semantics and Actus source behavior do not change.
- Ownership, cleanup, effect, capability, ABI, and backend checks run exactly
  as required by the selected compilation path.
- Fault fixtures remain isolated and cannot poison cached artifacts.
- No test may observe another test's mutable filesystem, environment, runtime,
  or native global state unless that sharing is already part of its contract.
- Output order and exit status remain deterministic.
- Cache invalidation is conservative: a false miss is acceptable; a false hit
  is not.
- Parallel workers do not write to the same mutable output path.

## Alternatives considered

### Compile and execute every test completely independently

Rejected as the only strategy because repeated module compilation and native
emission dominate suite time. It remains the correctness fallback.

### Run every test in one process

Rejected because process exit, fault injection, traps, cleanup failures, and
global native state require isolation. A single process would also make one
test's failure capable of hiding or contaminating later results.

### Use an unvalidated global object cache

Rejected because stale objects could silently mix compiler versions, target
contracts, generic instances, runtime bridges, or backend settings. Cache
identity must be explicit and conservative.

### Make output faster by removing diagnostics or progress reporting

Rejected. Reduced output is not a performance solution and would make long
compilation stages appear hung. The runner should expose stage progress and
timings without changing diagnostic content.

## Acceptance evidence

The implementation is accepted only when all of the following are demonstrated:

- the existing full strict suite has the same discovered test count and stable
  ordering before and after optimization;
- focused fault, signal, cleanup, timeout, and abnormal-exit tests preserve
  their current isolation and outcomes;
- repeated runs produce equivalent pass/fail classifications and diagnostics;
- cache hits never reuse an incompatible object, and conservative invalidation
  is covered by regression tests;
- bounded parallel preparation does not race or mutate shared compiler state;
- wall time, CPU time, peak temporary storage, and cache hit/miss counts are
  measured on a documented host and compiler build; and
- `cargo fmt`, `cargo clippy`, and the relevant test suites pass, while any
  unrelated pre-existing failure is reported separately rather than hidden.

## Consequences

The runner will have a faster preparation path and can avoid repeated native
module emission without weakening Actus's correctness boundary. The design
adds cache-key and lifecycle complexity, so the first implementation favors
conservative misses and explicit instrumentation over aggressive grouping.
Future single-executable grouping remains possible, but only after its process
and failure semantics are proven with dedicated tests.
