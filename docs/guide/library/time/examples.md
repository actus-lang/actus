# `std::time` examples

## Measure elapsed time

```actus
import std::time;

open verb elapsed_work() -> Result[Duration, TimeError] {
    erg before: Instant = now();
    # bounded work
    return duration_since(later: now(), earlier: abs before);
}
```

The result is valid only inside the same monotonic provider domain.

## Exact unit arithmetic

```actus
open verb two_seconds() -> Result[Duration, TimeError] {
    erg one = duration_seconds(seconds: 1u64)?;
    return duration_mul(duration: abs one, multiplier: 2u64);
}
```

Overflow and non-exact division remain typed failures.

## Deadline with remaining budget

```actus
open verb budgeted(abs budget: Duration) -> Result[Duration, TimeError] {
    erg deadline = deadline_from_now(duration: abs budget)?;
    return remaining(deadline: abs deadline);
}
```

An expired deadline returns an exact zero duration.

## Deterministic timer polling

```actus
open verb poll_at(abs start: Instant, abs now_point: Instant)
    -> Result[TimerPoll, TimeError] {
    erg timer: Timer = timer_one_shot();
    erg delay = duration_nanos(nanos: 10u64);
    timer_arm_at(timer: ins timer, start: abs start, delay: abs delay)?;
    return timer_poll_at(timer: ins timer, instant: abs now_point);
}
```

Supplying the instant explicitly makes timer behavior testable without waiting
on a real scheduler.

## Periodic catch-up

```actus
open verb periodic(abs start: Instant, abs period: Duration)
    -> Result[TimerPoll, TimeError] {
    erg timer: Timer = timer_one_shot();
    timer_periodic(
        timer: ins timer,
        period: abs period,
        policy: abs MissedTickPolicy.CatchUp,
    )?;
    timer_arm_at(timer: ins timer, start: abs start, delay: abs period)?;
    return timer_poll_at(timer: ins timer, instant: abs start);
}
```

`CatchUp` preserves the original periodic phase by advancing the prior
deadline rather than restarting from the current poll instant.

## Short delay versus scheduler sleep

```actus
open verb short_wait(abs duration: Duration) -> Result[Int, TimeError] {
    return delay(duration: abs duration);
}

open verb cooperative_wait(abs duration: Duration) -> Result[Int, TimeError] {
    return sleep(duration: abs duration);
}
```

The first can occupy the core. The second delegates blocking or yielding to
the active runtime provider.
