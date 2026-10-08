# Time errors

## `NegativeDuration`

The later instant precedes the earlier instant, or subtraction would require a
negative unsigned duration. No underflow or wrapping is returned.

## `Overflow`

A unit conversion, duration arithmetic operation, deadline addition, or
periodic timer rescheduling would exceed `u64`.

## `ZeroDivisor`

`duration_div` received zero. The operation does not invoke an undefined
integer division.

## `PrecisionLoss`

A conversion or division would discard non-zero lower-order nanoseconds. The
library requires exact conversion and asks the caller to choose a unit or
rounding policy explicitly.

## `ProviderUnavailable`

The selected runtime does not provide the requested monotonic sleep or other
provider-backed operation. This is a capability boundary, not a duration
value.

## `InvalidTimerPeriod`

A periodic timer was configured with a zero duration. A periodic timer needs a
positive interval to make progress.

## `InvalidTimerState`

A lifecycle operation was incompatible with the timer's current state, such
as polling a newly created timer, configuring an already used timer, or
canceling a completed timer.

## Handling rule

Branch on the typed result and preserve the distinction between overflow,
precision, provider availability, and lifecycle state. Do not convert a time
error into zero duration or pretend that an unarmed timer is pending.
