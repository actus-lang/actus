# Moves and scalar reuse

A move transfers ownership and invalidates the previous owner for the moved
resource. The compiler rejects later use of the moved binding.

A move may happen through a `dat` call, a returned owned value, an assignment,
or a payload operation. Fields and indexed places are tracked as part of their
containing owner.

## Scalar reuse

Eligible scalar values can be reused explicitly:

```actus
verb read(abs value: u32) -> Int {
    return value as Int;
}

verb main() -> Int {
    erg value: u32 = 41u32;
    return read(value: abs value);
}
```

Use `copy(value: abs scalar)` only for supported scalar values. It does not
make buffers, strings, resources, or cleanup-bearing aggregates copyable.
Identity arithmetic such as `value + 0u32` is not a substitute for explicit
reuse.

A value that is borrowed or moved cannot be reused until the relevant loan or
ownership transition has ended.
