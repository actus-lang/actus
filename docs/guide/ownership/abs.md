# `abs`: read-only view

`abs` is a shared, read-only view. It lets a callee inspect an existing value
without taking ownership or scheduling its cleanup.

```actus
verb checksum(abs input: Buffer) -> u16 {
    return crc16(input: abs input);
}
```

The source owner remains responsible for the value. While the view is active,
operations that would invalidate or mutate the owner are restricted.

Use `abs` for inspection, comparisons, encoding from existing data, and public
APIs that do not need to modify or consume the input.

An `abs` view cannot be used as an `erg` owner, a `dat` transfer, or an `ins`
loan. If a callee needs one of those meanings, declare that contract explicitly.
