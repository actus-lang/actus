# `std::time`

`std::time` provides integer-only monotonic timing. It covers provider ticks,
immutable instants, exact nanosecond durations, checked arithmetic, deadlines,
busy-wait delay, scheduler-aware sleep, and bounded one-shot or periodic
timers.

```actus
import std::time;
```

The module is about elapsed time and scheduling boundaries. It does not expose
calendar dates, wall-clock timestamps, time zones, formatting, raw platform
clock structures, callbacks, or hidden timer allocation.

## Reading order

1. [API](api.md) — public types, verbs, and result mapping.
2. [Duration](duration.md) — units, exact conversion, arithmetic, and overflow.
3. [Instants and deadlines](instants-and-deadlines.md) — monotonic ordering and
   expiration.
4. [Timers](timers.md) — lifecycle, polling, periodic policy, and cancellation.
5. [Delay and sleep](delay-and-sleep.md) — busy-wait versus provider sleep.
6. [Errors](errors.md) and [runtime](runtime.md).
7. [Usage](usage.md) and [examples](examples.md).

## Core model

- `monotonic_nanos()` reads a provider-domain counter.
- `Instant` names one point in that same domain.
- `Duration` stores a non-negative span exactly in nanoseconds.
- `Deadline` stores a future provider-domain tick boundary.
- `Timer` is bounded scalar state, not a registered OS object or callback.

All arithmetic is integer-based. Checked operations return typed errors rather
than wrapping, silently truncating, or clamping values.

## Provider domain

An instant, deadline, or timer deadline is meaningful only in the monotonic
provider domain that created it. The numeric origin is unspecified and is not
an epoch. Use the public operations for ordering and subtraction; do not
serialize ticks as wall-clock timestamps.

## Availability and blocking

The hosted profile supplies the monotonic provider and scheduler-aware sleep.
Unsupported freestanding profiles must reject unavailable operations at their
runtime boundary. `delay` busy-waits and can occupy a core; `sleep` may block
or yield through the selected provider. Neither operation promises an exact
wake-up instant.

## Source and evidence

The implementation is in
[`library/std/src/time/`](../../../../library/std/src/time/). Native and
semantic coverage is in [`tests/std_time_native.rs`](../../../../tests/std_time_native.rs),
[`tests/std_time_semantic.rs`](../../../../tests/std_time_semantic.rs), and the
timer fixture [`tests/fixtures/std_time_timer.act`](../../../../tests/fixtures/std_time_timer.act).
