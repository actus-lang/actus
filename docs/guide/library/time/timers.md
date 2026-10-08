# Timers

`Timer` is a bounded value-state machine. It stores scalar lifecycle and
provider-domain deadline data; it does not register a callback, allocate a
heap object, or own a scheduler entry.

## States

- `Created`: constructed but not armed.
- `Armed`: has a deadline eligible for polling.
- `Expired`: one-shot fired and awaits the terminal completion observation.
- `Canceled`: cancellation was accepted; later polls report `Canceled`.
- `Completed`: one-shot completion was observed; later arm/poll operations are
  invalid or report `Completed` according to the verb.

## One-shot lifecycle

```actus
erg timer: Timer = timer_one_shot();
timer_arm(timer: ins timer, delay: abs duration);
```

Before the deadline, `timer_poll` returns `Pending`. At or after the deadline,
it returns `Fired` and moves to `Expired`. The next poll returns `Completed` and
moves to `Completed`. A timer in `Expired` may be rearmed before completion is
observed; a completed timer cannot be rearmed.

`timer_arm_at` accepts an explicit `Instant`, which is useful for deterministic
provider tests and embedded integrations. `timer_arm` captures `now()` first.

## Periodic configuration

Configure a fresh timer with a positive period:

```actus
erg timer: Timer = timer_one_shot();
erg configured = timer_periodic(
    timer: ins timer,
    period: abs period,
    policy: abs MissedTickPolicy.CatchUp,
);
```

A zero period returns `InvalidTimerPeriod`. Configuration of a timer outside
`Created` returns `InvalidTimerState`.

## Missed-tick policies

When a periodic timer is overdue, each policy returns at most one poll event:

- `Coalesce`: return `Fired` and schedule the next deadline from the current
  instant. Missed periods collapse into one event.
- `CatchUp`: return `Fired` and advance the previous deadline by one period,
  preserving phase for later polls.
- `Skip`: return `Pending` and schedule the next deadline from the current
  instant. Overdue work is discarded.

A periodic timer remains `Armed` after each policy decision. Overflow while
computing its next deadline returns `TimeError.Overflow`.

## Polling

`timer_poll` reads the active provider. `timer_poll_at` receives an explicit
instant and is the deterministic testing and target-provider boundary. Neither
operation invokes user callbacks or performs allocation.

## Cancellation

`timer_cancel` moves any non-completed timer to `Canceled` and returns `Ok(0)`.
Canceling an already canceled timer is idempotent. Canceling a completed timer
returns `InvalidTimerState`. A canceled timer never fires again.

## State inspection

`timer_state(abs timer)` returns a value without mutating or consuming the
timer. Keep it as an observation only; lifecycle changes must go through the
arm, poll, and cancel verbs.
