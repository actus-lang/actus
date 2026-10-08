# `std::time`

```actus
import std::time;
```
`std::time` provides monotonic instants, exact integer durations, deadlines,
busy-wait delays, provider-backed sleep, and one-shot or periodic timer state
machines.

The module does not represent wall-clock time, calendars, time zones, or raw
platform clock structures. Duration construction and arithmetic use checked
integer units. Underflow, overflow, invalid deadlines, and unsupported runtime
providers produce typed results or diagnostics according to the API.

Runtime availability is selected by the package profile. Do not use a hosted
time provider in a freestanding program without an explicit target contract.

## Public surface

The facade includes:

- `Instant`, `now`, `instant_ticks`, `monotonic_nanos`, `duration_since`, and
  `elapsed_since`;
- `Duration` construction, conversion, addition, subtraction, multiplication,
  and division;
- `Deadline`, deadline construction, `expired`, and `remaining`;
- `delay`, `sleep`, and deadline waiting;
- `Timer`, one-shot and periodic construction, arm, poll, cancel, and state
  operations;
- `TimeError` variants for invalid ranges, overflow, underflow, and provider
  failures.

Time values are inspected through `abs`; timers are changed through `ins`.
Duration arithmetic returns typed failures where the result cannot be
represented.

## Detailed pages

- [Usage guide](usage.md)
- [Time implementation](../../../../library/std/src/time/)
- [Time tests](../../../../tests/std_time_native.rs)

## Choosing an operation

- Use `now` and `duration_since` for elapsed monotonic measurement.
- Use `Deadline` when a later operation must be bounded by an absolute
  monotonic point.
- Use `delay` for an explicit busy-wait and `sleep` when the runtime provider
  can suspend execution.
- Use `Timer` when polling a one-shot or periodic state machine and choose the
  missed-tick policy explicitly.

The numeric unit is part of the type contract. Do not compare raw tick values
from unrelated providers or treat monotonic time as a calendar timestamp.
