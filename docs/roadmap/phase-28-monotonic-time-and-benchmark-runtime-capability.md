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

- [x] Inventory existing standard-library facades, runtime profiles, external
      bridge declarations, native symbol bindings, and target policy checks.
- [x] Identify the current hosted and freestanding runtime boundaries.
- [x] Define the stable diagnostic for unavailable monotonic time.
- [x] Define overflow behavior when a platform timestamp cannot be represented
      as nanoseconds in `u64`.
- [x] Record the API, ABI, ownership, allocation, and precision boundaries in
      this roadmap before implementation.

### Gate 28.0 evidence

The audit confirms the following existing boundaries:

- `library/std/src/lib.act` is the standard-library root facade. The existing
  module facades are `io/io.act`, `fs/fs.act`, and `path/path.act`; their
  builtin registration and target classes are declared in
  `library/std/Actus.toml`.
- `RuntimeProfile` is the closed manifest-level profile set in
  `src/configuration/manifest.rs`: `core`, `std`, and `freestanding`. Omitted
  runtime selects `core`; target entry contracts are derived by
  `src/target.rs` and validated during configuration loading.
- Runtime module target compatibility is already enforced by the module
  resolver. A builtin module that exists but is unavailable for the selected
  target uses the stable module diagnostic `E1112` (`IncompatibleRuntime`),
  before native emission. `std::time` will use this same unavailable-module
  contract rather than inventing a second visibility path.
- Native external declarations and symbol bindings are owned by the existing
  `src/codegen/native` and `src/runtime` boundaries. The timer bridge must be
  registered there, not through a special case in expression lowering.
- The hosted boundary is the `Hosted` entry contract and the existing hosted
  runtime bridges. Freestanding/core builds do not inherit a host clock; a
  future explicit provider must be declared by runtime/target metadata before
  native lowering.

The Phase 28 failure policy is now fixed:

- unavailable `std::time` on the selected runtime/target is rejected with
  `E1112` during module/runtime compatibility validation;
- a hosted provider timestamp that cannot be checked-converted to nanoseconds
  in `u64` is a hard runtime-contract failure, never wrapping, clamping, or
  truncating. The bridge implementation will expose that failure through the
  existing typed runtime failure boundary and reserve `E1899` for the
  deterministic execution diagnostic when the failure reaches native runtime;
- the public API remains exactly `monotonic_nanos() -> u64`, with no pointer,
  platform structure, allocation, wall-clock semantics, or floating-point
  operation.

This closes the audit gate with architecture evidence only. No timer facade,
bridge, or runtime implementation is claimed complete until Gates 28.1–28.4
provide direct source, native, and execution evidence.

## Gate 28.1: Public `std::time` API and facade

- [x] Add the canonical `std::time` directory facade and sibling declaration
      according to standard-library module rules.
- [x] Declare `open verb monotonic_nanos() -> u64` with complete Actus
      documentation.
- [x] Keep the Actus-facing declaration independent of C types, pointers,
      `timespec`, `Instant`, or platform-specific structures.
- [x] Define the public behavior for repeated reads, process lifetime, and
      `u64` representability.
- [x] Add accepted and rejected semantic fixtures for importing and calling
      the API.

### Gate 28.1 evidence

- Added `library/std/src/time/time.act` as the canonical facade and
  `library/std/src/time/monotonic.act` as its sibling declaration.
- Registered `time` as a hosted builtin module in `library/std/Actus.toml`.
- Exposed only `monotonic_nanos() -> u64`; the raw
  `actus_monotonic_nanos() -> u64` bridge remains private to the module.
- Documented non-decreasing sequential reads, unspecified monotonic origin,
  lack of wall-clock meaning, no allocation, no platform type exposure, and
  no one-nanosecond resolution guarantee.
- Added semantic fixtures for valid import/call usage and rejection of the
  private bridge. Added a runtime-plan fixture proving canonical
  `import std::time;` resolves to the builtin facade.
- Verified the public facade and sibling module with the documentation and
  module visibility suites. Native linkage is intentionally deferred to
  Gate 28.2.

## Gate 28.2: Hosted runtime bridge

- [x] Add the documented native bridge for the hosted `std` runtime.
- [x] Use a platform monotonic clock rather than wall-clock time.
- [x] Convert the platform result to nanoseconds with checked arithmetic.
- [x] Return a deterministic typed failure or target diagnostic if the clock
      cannot be read or its value cannot fit in `u64`.
- [x] Keep the bridge allocation-free and free of floating-point operations.
- [x] Register the bridge through the existing runtime/native symbol boundary;
      do not add special handling to unrelated code-generation paths.

### Gate 28.2 evidence

- Added `src/runtime/time.rs` with a process-local `OnceLock<Instant>` origin
  and a hosted `actus_monotonic_nanos() -> u64` C-ABI symbol.
- The bridge uses `Instant`, never wall-clock/calendar APIs, and converts
  `Duration::as_nanos()` through checked `u64::try_from`.
- Conversion overflow is not wrapped, clamped, or truncated; the bridge uses
  the documented hard runtime failure boundary. The overflow conversion path
  has a direct regression test.
- Registered the stable symbol in `src/runtime/contract.rs` and exported it
  through `src/runtime/mod.rs`; no code-generation special case was added.
- Added native hosted acceptance coverage that builds and executes a package
  importing `std::time`, performs two reads, enables `verify_no_float_ir`, and
  requires exit code `0` with empty output.
- Runtime and semantic evidence passed: 10 runtime tests, 3 `std::time`
  semantic tests, and the hosted native executable test.

## Gate 28.3: Compiler semantic and native lowering support

- [x] Register the `std::time` declaration in the same semantic and module
      visibility path as other standard-library APIs.
- [x] Lower the call with the declared `u64` return type and existing native
      call conventions.
- [x] Preserve the call through imported facades and nested package module
      graphs.
- [x] Ensure native dependency closure includes the timer bridge exactly once.
- [x] Reject unavailable runtime/target combinations before native emission.
- [x] Verify generated integer IR contains no floating-point instructions.

### Gate 28.3 evidence

- The existing semantic/module resolver path resolves canonical
  `import std::time;` imports and exposes only the facade's public verb.
- The existing external-verb lowering path preserves the declared `u64`
  return ABI without adding a time-specific expression-lowering branch.
- Hosted object acceptance verifies that the undefined canonical `std::time`
  facade wrapper appears exactly once in the emitted object, while executable
  acceptance proves its transitive `actus_monotonic_nanos` runtime link
  resolves.
- A freestanding `std` project importing `std::time` is rejected with
  `E1112` during target/module validation, before native emission.
- The hosted acceptance manifest enables `verify_no_float_ir`; the build and
  generated IR audit pass with no floating-point instructions.

## Gate 28.4: Monotonicity, arithmetic, and runtime safety

- [x] Add a native ordering test proving `finished >= started` for sequential
      reads in one process.
- [x] Add a repeated-read test that does not assume a specific clock
      resolution or exact elapsed duration.
- [x] Test elapsed subtraction with `u64` values in the language.
- [x] Define and test behavior for a counter conversion overflow.
- [x] Verify no negative duration is representable through the public unsigned
      API without an explicit source-level underflow diagnostic or trap.
- [x] Verify the bridge does not allocate and does not expose platform state.

### Gate 28.4 evidence

- Hosted native acceptance performs two reads, computes `finished - started`
  as `u64`, and checks the ordering without assuming a specific resolution or
  elapsed duration.
- Runtime unit coverage proves sequential reads are non-decreasing and that
  nanosecond conversion rejects a value beyond `u64` instead of truncating it.
- A source fixture with unsigned elapsed underflow cannot produce a successful
  native execution; the runtime terminates with a non-zero failure at the
  native trap boundary.
- The bridge stores only a process-local `Instant` in a static `OnceLock`,
  performs no heap allocation or platform-state exposure in its Actus-facing
  ABI, and uses no floating-point operations.

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

## Expanded `std::time` library contract

The initial `monotonic_nanos()` capability is only the lowest-level clock
primitive. It is not the complete Actus time library. The remaining work in
this phase expands `std::time` into a typed, integer-only timing surface that
is usable by hosted applications, embedded firmware, and future scheduler
implementations without exposing host-specific clock structures.

The library is divided into these public responsibilities:

| Area | Public responsibility |
| --- | --- |
| `Instant` | An opaque monotonic point suitable for ordering and elapsed-time calculations. |
| `Duration` | A bounded unsigned span with checked construction and arithmetic. |
| Units | Explicit nanosecond, microsecond, millisecond, and second conversions. |
| Deadlines | Expiration and remaining-time calculations without wall-clock semantics. |
| Delays | Explicit blocking delay contracts for hosted and freestanding providers. |
| Timers | One-shot and periodic timer contracts with deterministic state transitions. |
| Providers | Manifest-selected clock, delay, and timer capabilities for each target. |
| Safety | Overflow, underflow, resolution, interrupt, and allocation guarantees. |

The public API must remain integer-only and must not expose `Instant` as a raw
platform timestamp. A provider may use a hardware counter, an OS clock, or a
host monotonic source internally, but the Actus-facing types and behavior are
the same.

### Required public model

The canonical Actus surface will be built incrementally around these types:

```act
import std::time;

erg started: Instant = now();
erg budget: Duration = milliseconds(10u64);
erg deadline: Deadline = deadline_after(budget: abs budget);

if expired(deadline: abs deadline) {
    return;
}

erg elapsed: Duration = elapsed_since(end: abs started);
```

The exact declaration names may be refined during implementation, but the
following semantic rules are fixed:

- `Instant` is monotonic and can only be created by the selected provider or
  by a documented test provider; users cannot construct an arbitrary instant
  from a wall-clock integer.
- `Duration` represents a non-negative bounded span. Its storage width and
  conversion behavior are explicit and stable; no signed wraparound is
  allowed.
- `Deadline` is an immutable monotonic expiration point derived from an
  `Instant` and a `Duration`. It is not a calendar timestamp.
- elapsed and remaining-time operations return typed results where overflow,
  underflow, or an unavailable provider is possible. They must not silently
  clamp or wrap values.
- constructors and arithmetic must preserve ownership roles and must not
  allocate.
- unit names must be explicit. No API may infer whether an integer means
  seconds, milliseconds, microseconds, or nanoseconds.

### Gate 28.7: Typed instant and duration model

- [x] Add the public `Instant`, `Duration`, and `TimeError` declarations to
      `library/std/src/time/` with complete ownership and error documentation.
- [x] Add a canonical `now()` operation that returns `Result[Instant,
      TimeError]` or another explicitly approved typed provider result; do not
      hide provider failure in a sentinel integer.
- [x] Add `elapsed_since` and `duration_since` operations with explicit
      earlier/later ordering behavior.
- [x] Define whether `Instant` and `Duration` are opaque structs, packs, or
      another stable Actus representation before native lowering is added.
- [x] Keep the raw counter and platform bridge private to the standard-library
      module.
- [x] Add accepted and rejected semantic tests for construction, ownership,
      ordering, invalid source types, and unavailable providers.

### Gate 28.7 evidence

- `library/std/src/time/instant.act` now defines the public `Instant` value,
  `now()`, `instant_ticks()`, `duration_since()`, and `elapsed_since()` verbs.
- `library/std/src/time/duration.act` defines the public nanosecond-backed
  `Duration`, its exact constructor, and its read-only accessors.
- `library/std/src/time/errors.act` defines typed negative-duration,
  overflow, and zero-divisor errors for the expanded API.
- The raw hosted bridge remains private and is reachable only through the
  public `std::time` facade.
- Semantic facade/import tests and hosted native tests pass; the native fixture
  captures two typed instants, reads their ticks, constructs a duration, and
  verifies the result with zero-float IR enabled.

### Gate 28.8: Duration units and checked arithmetic

- [x] Add explicit constructors for seconds, milliseconds, microseconds, and
      nanoseconds.
- [x] Add checked conversion operations between all supported units.
- [x] Add checked addition, subtraction, multiplication by an unsigned scalar,
      and division by a non-zero unsigned scalar.
- [x] Define and test overflow, underflow, zero divisor, and precision-loss
      behavior with typed diagnostics or typed errors.
- [x] Do not silently round, truncate, wrap, or promote a duration across a
      unit boundary.
- [x] Add native tests for boundary values and verify integer-only IR.

### Gate 28.8 evidence

- `library/std/src/time/duration.act` now provides exact nanosecond,
  microsecond, millisecond, and second constructors, checked conversions, and
  checked arithmetic over `u64` nanoseconds.
- Overflow, unsigned underflow, zero divisors, and non-exact conversions are
  represented by `TimeError`; division and unit conversion never silently
  discard elapsed-time precision.
- `duration_boundaries_and_typed_failures_run_natively` covers maximum valid
  seconds, constructor overflow at each larger unit, exact division,
  precision loss, zero division, and subtraction underflow in a native
  executable.
- The native boundary test passes with the no-floating-point IR audit enabled.

### Gate 28.9: Deadlines and remaining-time calculations

- [x] Add immutable deadline construction from an instant and duration.
- [x] Add `expired`, `remaining`, and deadline comparison operations.
- [x] Define behavior for an already expired deadline, zero duration, and
      duration addition overflow.
- [x] Ensure deadline calculations use the same provider and monotonic domain
      as the originating instant.
- [x] Reject mixing values from incompatible provider domains or target clock
      epochs if the implementation exposes multiple domains.
- [x] Add deterministic native acceptance tests.
- [x] Add explicit native rejection coverage for deadline overflow and
      incompatible provider domains.

### Gate 28.9 evidence

- `library/std/src/time/deadline.act` defines `Deadline`, checked construction
  from an `Instant` and `Duration`, relative construction, expiration, and
  remaining-time operations.
- Deadline addition rejects `u64` overflow and expiration returns zero
  remaining duration after the current monotonic point reaches the deadline.
- Native acceptance captures a one-second deadline in the active provider
  domain, verifies it has not already expired, and passes the zero-float IR
  audit.
- `deadline_overflow_is_rejected_natively` proves that adding a non-zero
  duration to the maximum provider tick returns `TimeError.Overflow` in an
  executable.
- The implementation exposes one active monotonic provider domain, so there
  are no cross-domain values to mix. The domain rule is explicit: values from
  another provider or clock epoch cannot be constructed through this API and
  must be rejected when multiple providers are introduced.

### Gate 28.10: Delay and sleep provider contracts

- [ ] Define separate contracts for a busy-wait delay and a scheduler-aware
      sleep; they must not be represented by one ambiguous verb.
- [ ] Add hosted implementations only where the selected runtime guarantees a
      monotonic delay provider.
- [ ] Define freestanding provider requirements for early boot, interrupt
      context, power state, and maximum blocking duration.
- [ ] Reject blocking sleep from interrupt/critical-section contexts when the
      target contract does not permit it.
- [ ] Specify whether a delay is best-effort, minimum-duration, or exact; the
      API must not promise stronger timing than the target can provide.
- [ ] Add tests proving no allocation, no floating-point operations, and
      deterministic unavailable-provider diagnostics.

### Gate 28.11: One-shot and periodic timers

- [ ] Define an explicit timer state model: created, armed, expired, canceled,
      and completed where applicable.
- [ ] Add one-shot timer creation and cancellation with ownership-safe handles.
- [ ] Add periodic timer creation with an explicit missed-tick policy:
      coalesce, catch-up, or skip; no implicit policy is allowed.
- [ ] Define timer callback/notification boundaries without requiring hidden
      heap allocation or dynamic dispatch in the core API.
- [ ] Define behavior for cancellation races, deadline overflow, provider
      resolution, and timer reuse.
- [ ] Keep scheduler integration separate from the core clock and duration
      types; `std::time` must not silently become an operating-system scheduler.
- [ ] Add deterministic fake-provider tests before hosted or hardware timing
      tests.

### Gate 28.12: Target provider and embedded contracts

- [ ] Define a manifest-driven provider contract without hardcoding CPU names
      into the language grammar or AST.
- [ ] A provider declaration must identify clock unit, counter width, wrap
      behavior, frequency conversion, read atomicity, interrupt safety, and
      initialization requirements.
- [ ] Define counter-wrap extension rules for bounded hardware counters and
      reject ambiguous wrap configurations.
- [ ] Define behavior on clock calibration changes, sleep/resume, reset, and
      tick discontinuity.
- [ ] Specify the minimum guarantees for ARM Cortex-M, ARM Cortex-A embedded,
      RISC-V MCU, and hosted profiles through target manifests/providers rather
      than compiler-specific CPU branches.
- [ ] Add provider conformance tests that run against a deterministic fake
      provider and a hosted provider; hardware tests remain target-specific.

### Gate 28.13: Tooling, examples, and complete library acceptance

- [ ] Add formatter and LSP support for every public time declaration,
      constructor, unit, deadline, delay, and timer operation.
- [ ] Add a general executable example that demonstrates instant, duration,
      conversion, deadline, and failure handling without any domain-specific
      engine.
- [ ] Add accepted and rejected tests for every public API and every provider
      profile.
- [ ] Require the same module/facade/visibility contract in `check`, `test`,
      object emission, executable emission, and LSP analysis.
- [ ] Verify no raw bridge symbol, platform type, or private helper is exposed
      through `std::time`.
- [ ] Update the coding guide, language guide, runtime documentation, ADR, and
      standard-library API index with the final declarations and guarantees.
- [ ] Pass all repository quality checks plus native, zero-float, allocation,
      fake-provider, hosted-provider, and target-rejection evidence.

## Non-goals

This phase does not:

- implement an AIE or any other application engine;
- define a global benchmark methodology for a separate project;
- provide wall-clock, calendar, or timezone APIs; those belong to a separate
  civil-time capability;
- turn the core clock into an implicit operating-system scheduler; delay,
  sleep, and timer APIs are explicit provider-backed contracts;
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
