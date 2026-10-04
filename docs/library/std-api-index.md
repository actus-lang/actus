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
