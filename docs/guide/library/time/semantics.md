# `std::time` semantic contract

## Monotonic meaning

`std::time` measures elapsed spans on one active monotonic provider. It does
not represent civil time. A tick has no public epoch, timezone, calendar, or
portable cross-process meaning.

The provider must return non-decreasing successful reads in its domain. Code
that needs elapsed time should capture an earlier `Instant` and use
`duration_since` or `elapsed_since`, rather than subtracting unrelated raw
values.

## Integer and precision rules

`Duration` is an exact unsigned nanosecond count. Unit construction checks
multiplication overflow. Conversion to seconds, milliseconds, or microseconds
requires exact divisibility. The library returns `PrecisionLoss` instead of
silently dropping nanoseconds.

Addition, multiplication, deadline construction, and periodic rescheduling
check `u64` bounds. Subtraction and reversed instants cannot underflow.

## Provider and blocking rules

The selected runtime supplies clock and sleep capability. `sleep` may block or
yield and has provider-dependent wake-up latency. `delay` is a busy-wait and
may keep the current core occupied. Neither is an exact calendar alarm.

Do not call scheduler-aware sleep from interrupt or critical-section code
unless the target provider explicitly allows it. Keep all timing assumptions
visible at the application boundary.

## Timer semantics

Timers use bounded scalar state and mutate through `ins`. Polling is explicit
and produces at most one notification per call. No callback, thread, heap
registration, or hidden scheduler object is created.

One-shot timers report `Pending`, then `Fired`, then `Completed` on the next
poll. Periodic timers stay armed and apply their selected missed-tick policy.
Cancellation is explicit and idempotent for an already canceled timer.

## Allocation and ABI

Instants, durations, deadlines, and timers contain only scalar fields. Their
operations do not allocate. Private provider bridges are hidden behind the
facade and expose no raw platform clock structures to Actus code.
