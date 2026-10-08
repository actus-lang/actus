# `std::time` public API

The public facade exposes typed Actus contracts. Provider bridge declarations
and configuration constants remain private to the time module.

## Types

```actus
struct Instant {
    ticks: u64,
}

struct Duration {
    nanos: u64,
}

struct Deadline {
    expires_at: u64,
}
```

These values contain bounded scalars and own no OS handle, scheduler entry,
callback, or heap allocation.

```actus
enum TimeError {
    NegativeDuration,
    Overflow,
    ZeroDivisor,
    PrecisionLoss,
    ProviderUnavailable,
    InvalidTimerPeriod,
    InvalidTimerState,
}
```

## Clock and elapsed time

```actus
verb monotonic_nanos() -> u64;
verb now() -> Instant;
verb instant_ticks(abs instant: Instant) -> u64;
verb duration_since(abs later: Instant, abs earlier: Instant)
    -> Result[Duration, TimeError];
verb elapsed_since(abs earlier: Instant) -> Result[Duration, TimeError];
```

The clock has no wall-clock meaning. `duration_since` rejects reversed order.

## Duration construction and conversion

```actus
verb duration_nanos(erg nanos: u64) -> Duration;
verb duration_as_nanos(abs duration: Duration) -> u64;
verb duration_value(abs duration: Duration) -> Result[u64, TimeError];
verb duration_seconds(erg seconds: u64) -> Result[Duration, TimeError];
verb duration_milliseconds(erg milliseconds: u64) -> Result[Duration, TimeError];
verb duration_microseconds(erg microseconds: u64) -> Result[Duration, TimeError];
verb duration_as_seconds(abs duration: Duration) -> Result[u64, TimeError];
verb duration_as_milliseconds(abs duration: Duration) -> Result[u64, TimeError];
verb duration_as_microseconds(abs duration: Duration) -> Result[u64, TimeError];
verb duration_add(abs left: Duration, abs right: Duration)
    -> Result[Duration, TimeError];
verb duration_sub(abs left: Duration, abs right: Duration)
    -> Result[Duration, TimeError];
verb duration_mul(abs duration: Duration, erg multiplier: u64)
    -> Result[Duration, TimeError];
verb duration_div(abs duration: Duration, erg divisor: u64)
    -> Result[Duration, TimeError];
```

Conversions to larger units require exact divisibility and return
`PrecisionLoss` rather than silently discarding lower-order units.

## Deadlines, delay, and sleep

```actus
verb deadline_after(abs start: Instant, abs duration: Duration)
    -> Result[Deadline, TimeError];
verb deadline_from_now(abs duration: Duration) -> Result[Deadline, TimeError];
verb expired(abs deadline: Deadline) -> Bool;
verb remaining(abs deadline: Deadline) -> Result[Duration, TimeError];
verb delay(abs duration: Duration) -> Result[Int, TimeError];
verb sleep(abs duration: Duration) -> Result[Int, TimeError];
```

`delay` busy-waits. `sleep` uses a scheduler-aware provider boundary.

## Timer types and verbs

```actus
enum TimerMode { OneShot, Periodic }
enum MissedTickPolicy { Coalesce, CatchUp, Skip }
enum TimerState { Created, Armed, Expired, Canceled, Completed }
enum TimerPoll { Pending, Fired, Canceled, Completed }

struct Timer {
    state: u8,
    mode: u8,
    policy: u8,
    period_nanos: u64,
    deadline_ticks: u64,
}
```

```actus
verb timer_one_shot() -> Timer;
verb timer_periodic(
    ins timer: Timer,
    abs period: Duration,
    abs policy: MissedTickPolicy,
) -> Result[Int, TimeError];
verb timer_arm(ins timer: Timer, abs delay: Duration)
    -> Result[Int, TimeError];
verb timer_arm_at(
    ins timer: Timer,
    abs start: Instant,
    abs delay: Duration,
) -> Result[Int, TimeError];
verb timer_poll(ins timer: Timer) -> Result[TimerPoll, TimeError];
verb timer_poll_at(ins timer: Timer, abs instant: Instant)
    -> Result[TimerPoll, TimeError];
verb timer_cancel(ins timer: Timer) -> Result[Int, TimeError];
verb timer_state(abs timer: Timer) -> TimerState;
```

The `ins` timer is mutated in caller-owned bounded storage. Polling never
invokes callbacks and emits at most one `Fired` notification per call.

## Result mapping

| Area | Success | Failure |
| --- | --- | --- |
| duration scale/arithmetic | exact value | `Overflow`, `PrecisionLoss`, `NegativeDuration`, `ZeroDivisor` |
| instant subtraction | `Duration` | `NegativeDuration` |
| deadline creation | `Deadline` | `Overflow` |
| delay/sleep | `Ok(0)` | `ProviderUnavailable` or overflow |
| timer configuration | `Ok(0)` | `InvalidTimerPeriod`, `InvalidTimerState` |
| timer polling | `TimerPoll` | `InvalidTimerState`, `Overflow` |
