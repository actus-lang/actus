# ADR-0056: Monotonic Time Runtime Capability

- Status: Proposed / Phase 28 contract
- Date: 2026-10-04
- Scope: `std::time`, hosted runtime timing, target availability, native ABI,
  compiler tooling, and benchmark evidence
- Depends on: ADR-0050 runtime profiles and builtin standard-library resolution

## Context

Actus currently has no public monotonic timer API. Applications can measure a
whole executable with an external host utility, but Actus code cannot measure
the elapsed duration of a bounded operation, compare two timestamps, or
produce a typed benchmark result from inside the language.

Wall-clock time is not an acceptable substitute. Calendar adjustments,
timezone configuration, synchronization corrections, and user changes can
make a wall clock move backwards or jump forwards. Elapsed-time measurement
requires a monotonic source whose value is suitable for subtraction within one
process.

The capability crosses several repository boundaries at once:

- the public standard-library facade and declaration;
- hosted runtime and native bridge registration;
- semantic availability and target/runtime policy;
- native `u64` return lowering and dependency closure;
- formatter, LSP, test runner, object, executable, and IR evidence.

Implementing this as an application-specific helper or an external timing
workaround would hide a missing language/runtime capability and make future
benchmark results depend on code outside Actus.

## Decision

Actus will provide a public hosted standard-library API:

```act
import std::time;

open verb monotonic_nanos() -> u64;
```

The actual public declaration is exposed by the canonical `std::time` facade.
The call returns a monotonic nanosecond tick count as an unsigned `u64`.
Actus code measures elapsed time by reading the value twice and subtracting the
earlier reading from the later reading:

```act
import std::time;

verb measure() -> u64 {
    erg started: u64 = monotonic_nanos();
    perform_work();
    erg finished: u64 = monotonic_nanos();
    return finished - started;
}
```

The API is an elapsed-time source, not a calendar or wall-clock API. It does
not expose a platform clock structure, raw pointer, timezone, timestamp
format, scheduler, sleep primitive, or required precision guarantee.

## Public contract

`monotonic_nanos()` must satisfy these conditions:

1. Sequential successful reads in one runtime instance are non-decreasing.
2. The value is an integer count expressed in nanoseconds from an unspecified
   monotonic origin.
3. The origin is not observable and must not be interpreted as Unix epoch time.
4. The return type is exactly `u64`; no floating-point conversion is allowed.
5. The call does not allocate and does not require caller-owned storage.
6. The API does not promise one-nanosecond hardware resolution or a fixed
   duration for any operation.
7. A benchmark must compare measurements made under the same target/runtime
   conditions; values from different machines are not directly comparable.

The standard library must document that timer resolution, scheduling noise,
clock source, and conversion behavior are platform-dependent even though the
Actus-facing type is stable.

## Hosted runtime and native ABI

The hosted `std` runtime supplies the first implementation through a host
monotonic clock. The platform-specific implementation remains behind the
existing runtime/native bridge boundary. Actus source sees only the typed
`u64` verb.

The bridge must:

- use a platform monotonic clock rather than a wall clock;
- perform checked conversion to nanoseconds and `u64`;
- report clock-read or conversion failure through the runtime's documented
  typed failure boundary, or fail the build when the target contract cannot
  represent the API;
- remain allocation-free and integer-only;
- be registered through normal native symbol/dependency resolution;
- be emitted once per native dependency closure.

The compiler must not special-case application names, benchmark consumers, or
domain modules to make the timer available.

## Freestanding and core policy

Freestanding and `core` profiles do not inherit a host clock. If the selected
runtime has no explicit monotonic-clock provider, importing or calling
`std::time` must be rejected with a stable diagnostic before native emission.

A future target runtime may provide the API only by declaring an explicit
provider contract in its runtime/target manifest. That provider must specify:

- the monotonic source and counter unit;
- conversion and overflow behavior;
- availability during early boot and interrupt contexts, if supported;
- ABI symbol ownership and linkage;
- whether the provider is allocation-free and integer-only.

The compiler must never link the hosted implementation into a freestanding
artifact implicitly.

## Overflow and failure policy

Platform counter conversion must use checked arithmetic. A result that cannot
be represented as nanoseconds in `u64` is not clamped, wrapped, or silently
truncated.

The implementation must choose one documented failure boundary for the
hosted runtime before implementation is accepted:

- return a typed runtime error through the standard-library surface; or
- reject the target/runtime contract before native code generation when failure
  cannot be represented by the current public signature.

The chosen policy must be identical in semantic checks, native lowering,
runtime behavior, and documentation. The initial implementation must not
invent an implicit sentinel timestamp.

For this phase, the failure boundary is fixed as follows:

- an unavailable runtime/target combination is rejected during module
  compatibility validation with the existing `E1112` incompatible-runtime
  diagnostic;
- a hosted provider conversion that cannot be represented as nanoseconds in
  `u64` is a hard typed runtime failure. It must not wrap, clamp, or truncate;
  if that failure reaches native execution, the deterministic execution
  diagnostic is reserved as `E1899`;
- the public signature remains `monotonic_nanos() -> u64`; no sentinel value
  is introduced to encode failure.

## Compiler and tooling obligations

The following components must consume one shared contract:

- module resolver and `std::time` facade discovery;
- semantic declaration and return-type checking;
- native lowering and dependency closure;
- runtime profile and target availability validation;
- formatter and source conformance;
- LSP completion, hover, definition, and diagnostics;
- strict check, test runner, object emission, and executable linking.

The generated IR for the timer call and its Actus arithmetic must pass the
existing zero-float audit. A passing parser or semantic check alone is not
evidence that the bridge is linked or that the returned value has the declared
ABI.

## Required evidence

Phase 28 must provide:

- accepted semantic and strict-check fixtures for hosted `std::time` use;
- rejected fixtures for unavailable freestanding/core runtime use;
- native ordering tests proving `finished >= started` without assuming exact
  clock resolution;
- `u64` elapsed subtraction and conversion-boundary tests;
- hosted object and executable tests;
- native dependency and symbol-resolution evidence;
- zero-float IR evidence;
- formatter, LSP, and test-runner parity evidence;
- a general Actus benchmark example, independent of AIE or another product.

## Consequences

Positive consequences:

- Actus programs can measure elapsed operations without shell workarounds;
- benchmark consumers use a stable integer API;
- wall-clock ambiguity is excluded from the public contract;
- hosted and freestanding runtime boundaries remain explicit;
- native, compiler, tooling, and documentation behavior can be tested together.

Costs and risks:

- the standard library needs a new canonical facade and documented bridge;
- target/runtime manifests must define availability rather than guessing;
- host clock resolution and scheduling noise limit benchmark comparability;
- overflow and runtime failure need an explicit public policy;
- LSP, formatter, native linker, and test runner must remain synchronized.

## Non-goals

This ADR does not introduce:

- wall-clock or calendar APIs;
- timezone or timestamp formatting;
- sleep, alarm, scheduler, or real-time guarantees;
- raw platform clock structures in Actus source;
- floating-point timing;
- implicit hosted timing in freestanding targets;
- AIE-specific or commercial-project-specific behavior.
