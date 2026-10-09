# Actus standard-library API index

This index lists the public standard-library facades available through the
selected `std` runtime. Internal facades, raw C bridges, and platform types are
not public APIs.

## `std::time`

`std::time` is an integer-only monotonic timing surface. It has no calendar,
timezone, or wall-clock meaning and does not allocate Actus storage.

- `Instant`, `Duration`, and `Deadline` are provider-domain value types.
- `TimeError` reports provider unavailability, overflow, precision loss, zero
  divisors, negative durations, invalid timer periods, and invalid timer state.
- `monotonic_nanos() -> u64` reads the active monotonic provider.
- `now() -> Instant` captures a provider-domain instant.
- `instant_ticks(abs Instant) -> u64` reads the value of an instant.
- `duration_nanos`, `duration_seconds`, `duration_milliseconds`, and
  `duration_microseconds` construct checked durations.
- `duration_as_nanos`, `duration_as_seconds`, `duration_as_milliseconds`, and
  `duration_as_microseconds` perform checked conversions.
- `duration_since` and `elapsed_since` perform checked instant subtraction.
- `duration_add`, `duration_sub`, `duration_mul`, and `duration_div` perform
  checked integer arithmetic.
- `deadline_after` and `deadline_from_now` construct monotonic deadlines.
- `expired` and `remaining` inspect deadlines without mutation.
- `delay` is an explicit busy-wait operation; `sleep` is an explicit
  scheduler-aware provider operation.
- `timer_one_shot`, `timer_periodic`, `timer_arm`, `timer_arm_at`,
  `timer_poll`, `timer_poll_at`, `timer_cancel`, and `timer_state` implement
  caller-owned one-shot and periodic timer state machines.

Hosted availability is selected by the runtime profile and provider contract.
Freestanding targets must declare a compatible provider; the compiler does not
silently link the hosted implementation.

## `std::io`, `std::fs`, and `std::path`

These existing facades expose typed ownership-aware I/O, filesystem, and path
operations. Their public surfaces are documented in the language guide and
their raw runtime bridges remain private.

## `std::region`

`std::region` is the public facade for a bounded logical storage resource. It
keeps a logical index space separate from its bounded resident window. It does
not convert an inline `Array[T, N]`, perform an implicit page fault, open a
file, allocate an unbounded collection, or expose a raw pointer.

### Current public types and errors

- `Region[T]` is an opaque, owned, compiler-defined runtime descriptor for a
  fully sized element type.
- `RegionError` is the typed failure enum for invalid descriptors, handles,
  generations, windows, offsets, buffers, capabilities, and provider failure.

### Current public operations

- `region_open[T](dat backing: Buffer, logical_length: u64,
  window_start: u64, window_count: u64) -> Result[Region[T], RegionError]`
  validates the element stride and opens one explicit resident window.
- `region_read[T](abs region: Region[T], index: u64,
  ins destination: Buffer) -> Result[Int, RegionError]` reads one resident
  element into an exact-size caller-owned buffer.
- `region_write[T](ins region: Region[T], index: u64,
  abs source: Buffer) -> Result[Int, RegionError]` writes one element and
  marks the resident state dirty.
- `region_publish[T](ins region: Region[T]) -> Result[u64, RegionError]`
  publishes the current resident bytes and advances the generation.
- `region_cancel[T](ins region: Region[T]) -> Result[Int, RegionError]`
  restores the last published resident bytes.
- `region_close[T](ins region: Region[T]) -> Result[Int, RegionError]`
  releases the capability exactly once.

The current public API provides one explicit resident window and lifecycle
operations. Window remapping, bounded range access, provider-neutral loading,
pinning, and recovery extensions are proposed by [ADR-0079](../decisions/ADR-0079-region-capability-completeness.md)
and are not part of the current compatibility baseline.

### Public boundary and private implementation

The canonical facade is `library/std/src/region/region.act`. The public
declarations are split between `error.act` and `api.act`. The
`actus_region_open`, `actus_region_read`, `actus_region_write`,
`actus_region_publish`, `actus_region_cancel`, and `actus_region_close`
declarations in `api.act` are private unsafe runtime bridges; applications
must use the typed public verbs above.

The complete Region contract and its planned extensions are recorded in
[ADR-0079](../decisions/ADR-0079-region-capability-completeness.md) and
[Phase 41](../roadmap/phase-41-region-capability-completeness.md).
