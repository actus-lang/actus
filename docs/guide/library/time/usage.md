# Using `std::time`

## Measure a bounded operation

```actus
import std::time;

open verb measure() -> Result[Duration, TimeError] {
    erg started: Instant = now();
    # perform the operation being measured
    return elapsed_since(earlier: abs started);
}
```

The result is a monotonic elapsed span. It has no calendar interpretation.

## Build an exact budget

```actus
erg budget_result = duration_milliseconds(milliseconds: 25u64);
return case dat budget_result {
    Result.Err(error) => Err(error),
    Result.Ok(budget) => use_budget(budget: abs budget),
};
```

Unit constructors check scaling overflow. Conversions back to larger units
require exact divisibility.

## Check a deadline without blocking

```actus
open verb still_allowed(abs deadline: Deadline) -> Bool {
    return !expired(deadline: abs deadline);
}
```

`remaining` returns zero after expiration, so callers can use it for a final
bounded wait decision.

## Use a one-shot timer

```actus
erg timer: Timer = timer_one_shot();
erg armed = timer_arm(timer: ins timer, delay: abs budget);
erg poll = timer_poll(timer: ins timer);
```

Handle `InvalidTimerState` if the surrounding lifecycle can poll or arm at an
unexpected point.

## Use a periodic timer

Configure a positive duration and select an explicit missed-tick policy. Use
`CatchUp` when phase matters, `Coalesce` when one notification is enough, and
`Skip` when overdue work should be discarded.

## Select delay or sleep

Use `delay` only for a short busy wait. Use `sleep` when the runtime should
cooperate with a scheduler or power-management layer. Neither is a precise
calendar alarm.
