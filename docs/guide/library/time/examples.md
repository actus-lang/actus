# `std::time` examples

## Measure elapsed monotonic time

```actus
import std::time;

verb elapsed_ns() -> Result[u64, TimeError] {
    erg start = now();
    erg finish = now();
    erg elapsed = duration_since(later: abs finish, earlier: abs start)?;
    return Result[u64, TimeError].Ok(duration_as_nanos(abs elapsed));
}
```

The result is a duration in the provider's monotonic nanosecond unit. It is
not a calendar timestamp.
