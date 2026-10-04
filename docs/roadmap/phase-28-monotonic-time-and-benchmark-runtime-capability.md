# Phase 28: Monotonic Time and Benchmark Runtime Capability

Phase 28 adds a first-class, integer-only monotonic timing capability to the
Actus standard runtime. It exists so Actus programs can measure elapsed
durations inside the language without relying on host shell utilities,
wall-clock time, or source-level workarounds.

This phase is a compiler and standard-runtime capability phase. It does not
implement an AIE engine, define a benchmark suite for a separate application,
or introduce domain-specific timing behavior. External systems may consume
the API after this phase is complete.

## Problem statement

Actus currently has no public monotonic timer API. A fixed workload can be
measured externally with `/usr/bin/time`, but that does not let Actus code
measure operation-level elapsed time, compare timestamps, or report a
benchmark result using the language's own typed arithmetic.

The capability must distinguish elapsed monotonic time from wall-clock time:

- it must not depend on timezone, calendar, or wall-clock adjustments;
- repeated reads must be non-decreasing for one process and runtime instance;
- the value must be an integer nanosecond count;
- the public Actus API must not expose raw pointers or platform-specific types;
- unsupported runtime profiles must fail explicitly rather than silently
  compiling a fake timer.

## Architectural contract

The public surface is:

```act
import std::time;

verb measure() -> u64 {
    erg started: u64 = monotonic_nanos();
    do_work();
    erg finished: u64 = monotonic_nanos();
    return finished - started;
}
```

The exact API contract is:

- module facade: `std::time`;
- public verb: `monotonic_nanos() -> u64`;
- return value: an unsigned nanosecond tick count from a monotonic source;
- no wall-clock, timezone, calendar, or formatting semantics;
- no allocation and no caller-provided storage;
- no floating-point operations in the Actus path or generated IR;
- no raw pointer or platform-specific type in the Actus-facing signature;
- the runtime bridge owns platform selection and error handling;
- the compiler and runtime must preserve the declared `u64` contract.

The API measures elapsed time. It does not promise nanosecond scheduling
precision, operation latency below one nanosecond, or identical values across
different machines.

## Runtime and target policy

Hosted `std` builds provide the initial implementation through the host's
monotonic clock. The implementation must use a platform monotonic source and
must convert its result to a checked `u64` nanosecond value.

Freestanding and `core` builds do not receive an implicit host clock. One of
the following explicit outcomes is required by target policy:

1. `std::time` is rejected with a stable diagnostic when the selected runtime
   has no monotonic clock contract; or
2. a target runtime manifest supplies an explicit monotonic-clock provider
   whose ABI and resolution contract are validated before native lowering.

The compiler must never link a hosted clock implementation into a
freestanding artifact by accident.

## Gate 28.0: Capability and target audit

- [ ] Inventory existing standard-library facades, runtime profiles, external
      bridge declarations, native symbol bindings, and target policy checks.
- [ ] Identify the current hosted and freestanding runtime boundaries.
- [ ] Define the stable diagnostic for unavailable monotonic time.
- [ ] Define overflow behavior when a platform timestamp cannot be represented
      as nanoseconds in `u64`.
- [ ] Record the API, ABI, ownership, allocation, and precision boundaries in
      this roadmap before implementation.

## Gate 28.1: Public `std::time` API and facade

- [ ] Add the canonical `std::time` directory facade and sibling declaration
      according to standard-library module rules.
- [ ] Declare `open verb monotonic_nanos() -> u64` with complete Actus
      documentation.
- [ ] Keep the Actus-facing declaration independent of C types, pointers,
      `timespec`, `Instant`, or platform-specific structures.
- [ ] Define the public behavior for repeated reads, process lifetime, and
      `u64` representability.
- [ ] Add accepted and rejected semantic fixtures for importing and calling
      the API.

## Gate 28.2: Hosted runtime bridge

- [ ] Add the documented native bridge for the hosted `std` runtime.
- [ ] Use a platform monotonic clock rather than wall-clock time.
- [ ] Convert the platform result to nanoseconds with checked arithmetic.
- [ ] Return a deterministic typed failure or target diagnostic if the clock
      cannot be read or its value cannot fit in `u64`.
- [ ] Keep the bridge allocation-free and free of floating-point operations.
- [ ] Register the bridge through the existing runtime/native symbol boundary;
      do not add special handling to unrelated code-generation paths.

## Gate 28.3: Compiler semantic and native lowering support

- [ ] Register the `std::time` declaration in the same semantic and module
      visibility path as other standard-library APIs.
- [ ] Lower the call with the declared `u64` return type and existing native
      call conventions.
- [ ] Preserve the call through imported facades and nested package module
      graphs.
- [ ] Ensure native dependency closure includes the timer bridge exactly once.
- [ ] Reject unavailable runtime/target combinations before native emission.
- [ ] Verify generated integer IR contains no floating-point instructions.

## Gate 28.4: Monotonicity, arithmetic, and runtime safety

- [ ] Add a native ordering test proving `finished >= started` for sequential
      reads in one process.
- [ ] Add a repeated-read test that does not assume a specific clock
      resolution or exact elapsed duration.
- [ ] Test elapsed subtraction with `u64` values in the language.
- [ ] Define and test behavior for a counter conversion overflow.
- [ ] Verify no negative duration is representable through the public unsigned
      API without an explicit source-level underflow diagnostic or trap.
- [ ] Verify the bridge does not allocate and does not expose platform state.

## Gate 28.5: Tooling and cross-profile behavior

- [ ] Add formatter coverage for `import std::time` and timer calls.
- [ ] Add LSP completion, hover, definition, and diagnostics for the public
      facade and verb.
- [ ] Add strict-check and test-runner coverage using the same module contract.
- [ ] Add hosted object and executable acceptance tests.
- [ ] Add freestanding/core rejected tests, unless an explicit target provider
      contract is implemented first.
- [ ] Verify the standard-library facade, compiler, linker, and LSP agree on
      visibility and unsupported-target behavior.

## Gate 28.6: Benchmark consumer contract and documentation

- [ ] Add a small general Actus timing example that reports elapsed nanoseconds
      without depending on a separate commercial or domain-specific project.
- [ ] Document a reproducible fixed-workload benchmark pattern using two timer
      reads and checked `u64` subtraction.
- [ ] Document that measurements are machine-, OS-, scheduler-, and
      resolution-dependent and must be compared under the same environment.
- [ ] Record object, executable, monotonic-ordering, zero-float, and target
      rejection evidence.
- [ ] Update `ACTUS_CODING_AGENT_GUIDE.md` and the relevant language/runtime
      documentation.
- [ ] Pass formatting, check, clippy, tests, source limits, architecture,
      documentation, LSP, and diff checks.

## Non-goals

This phase does not:

- implement an AIE or any other application engine;
- define a global benchmark methodology for a separate project;
- provide wall-clock, calendar, timezone, sleep, alarm, or scheduler APIs;
- promise nanosecond precision or real-time scheduling guarantees;
- expose raw platform clock structures to Actus code;
- add floating-point timing or formatting requirements;
- silently emulate a timer on unsupported freestanding targets.

## Completion criteria

Phase 28 is complete when hosted Actus programs can call
`std::time::monotonic_nanos()` with a stable `u64` contract, sequential reads
are proven non-decreasing, elapsed arithmetic is safe and tested, native
object/executable and zero-float evidence passes, unsupported targets fail
closed or provide an explicit provider contract, tooling agrees with the
compiler, and a general benchmark example documents the supported workflow.
