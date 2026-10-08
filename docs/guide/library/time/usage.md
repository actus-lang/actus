# `std::time` usage

## Monotonic measurement

Use `now` for an `Instant` and `duration_since` for checked elapsed time. An
`Instant` is ordered runtime time, not a wall-clock date.

## Durations

Construct durations with `duration_seconds`, `duration_milliseconds`, or
`duration_microseconds`. Use the conversion verbs to obtain an integer unit.
Addition, subtraction, multiplication, and division return a typed error when
the result overflows, underflows, or divides by zero.

## Deadlines and waiting

Use `deadline_after` or `deadline_from_now` to create a monotonic deadline.
`expired` is a pure query; `remaining` returns a checked `Duration`. Use
`delay` for a deliberate busy wait and `sleep` when the selected runtime has a
sleep provider.

## Timers

`Timer` is a caller-owned state machine. Construct one-shot or periodic mode,
arm it, poll it with an `Instant`, and handle `TimerPoll` according to the
selected missed-tick policy. `timer_cancel` ends the active schedule.
