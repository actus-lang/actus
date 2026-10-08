# Instants and deadlines

## Monotonic instants

`now()` captures an `Instant` from the active monotonic provider. The value is
ordered within that provider domain, but its origin is unspecified:

```actus
erg started: Instant = now();
# perform bounded work here
erg elapsed = elapsed_since(earlier: abs started);
```

`instant_ticks` exposes the provider tick only for same-domain diagnostics or
carefully controlled arithmetic. It is not a Unix timestamp, calendar value,
or portable cross-machine identifier.

## Elapsed time

`duration_since(later, earlier)` performs checked unsigned subtraction. If the
later instant precedes the earlier one, it returns `NegativeDuration` rather
than wrapping. Both instants must come from the same monotonic domain.

## Creating a deadline

`deadline_after` adds a duration to a supplied instant and returns
`Overflow` if the tick boundary cannot fit in `u64`. `deadline_from_now` uses
one current instant and the same checked addition.

```actus
erg timeout: Duration = duration_seconds(seconds: 1u64)?;
erg deadline = deadline_from_now(duration: abs timeout)?;
```

A `Deadline` is a scalar expiration boundary. It does not register a callback,
allocate an OS timer, or keep a scheduler resource alive.

## Expiration and remaining time

`expired` reads a new current instant and returns true at or after the boundary.
`remaining` returns zero for an already expired deadline and a non-negative
`Duration` otherwise. Neither operation mutates the deadline.

## Domain rule

Do not compare an instant from one provider with a tick captured from another
provider. The public API has one active domain per selected runtime profile;
the domain rule exists so a future target provider cannot make mixed-clock
arithmetic appear valid.
