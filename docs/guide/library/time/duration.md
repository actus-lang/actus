# Durations

`Duration` is a non-negative span stored as an exact `u64` nanosecond count.
It has no clock domain and does not represent a timestamp.

```actus
struct Duration {
    nanos: u64,
}
```

## Construction

`duration_nanos` accepts every `u64` count. The unit constructors multiply a
whole input by its exact scale and reject overflow:

```actus
open verb budget() -> Result[Duration, TimeError] {
    return duration_milliseconds(milliseconds: 25u64);
}
```

The scales are one thousand nanoseconds per microsecond, one million per
millisecond, and one billion per second. Fractional unit inputs are not
represented by these constructors.

## Reading a duration

`duration_as_nanos` returns the exact scalar directly. `duration_value` returns
the same value through `Result` for code that uses a uniform fallible value
surface. Neither operation consumes or mutates the duration.

## Exact conversion out

`duration_as_seconds`, `duration_as_milliseconds`, and
`duration_as_microseconds` require exact divisibility. For example, 1500
nanoseconds cannot be returned as whole microseconds because doing so would
lose 500 nanoseconds; the result is `TimeError.PrecisionLoss`.

This rule prevents accidental truncation in timeout and persistence logic.

## Arithmetic

- `duration_add` rejects sums beyond `u64`.
- `duration_sub` rejects a right operand larger than the left operand with
  `NegativeDuration`.
- `duration_mul` checks multiplication before performing it; zero is valid.
- `duration_div` rejects zero and rejects non-exact division.

The operations borrow their duration inputs with `abs`, do not mutate them,
and allocate no storage.

## Integer-only contract

All duration operations use integer nanoseconds and checked arithmetic. There
are no floating-point values, rounding modes, implicit saturation, or hidden
unit conversions.
