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
