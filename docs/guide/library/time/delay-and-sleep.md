# Delay and sleep

The library intentionally separates busy-wait delay from provider-backed
sleep. They have different scheduling and power behavior.

## `delay`

`delay(abs duration)` creates a monotonic deadline and loops until the provider
reaches it. It returns `Ok(0)` after the minimum duration has elapsed. It uses
no heap storage, callback, or scheduler entry, but it may monopolize the
current execution core.

Use it only for short bounded waits where busy-waiting is an explicit choice.
It is suitable for carefully controlled low-level timing but is a poor choice
for a long application timeout.

## `sleep`

`sleep(abs duration)` sends the exact nanosecond count to the selected runtime
provider. The provider may block or yield the current execution context. It
may wake later than requested and does not promise an exact wake-up instant.

Do not call scheduler-aware sleep from an interrupt handler or critical
section unless the selected target explicitly permits it. Unsupported profiles
return `ProviderUnavailable` at their defined capability boundary.

## Choosing between them

| Need | Operation |
| --- | --- |
| short deterministic busy wait | `delay` |
| scheduler cooperation or power saving | `sleep` |
| repeated event notifications | `Timer` |
| inspect a future boundary without waiting | `Deadline` |

Neither operation is a wall-clock alarm. For cancellation-aware application
work, keep the deadline visible and poll it from the surrounding scheduler.
