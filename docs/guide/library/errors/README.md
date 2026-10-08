# Standard-library errors and results

The standard library reports recoverable failure through typed
`Result[Success, Error]` values. A result is part of the public contract: its
error variants describe what the caller may safely retry, translate, recover,
or report.

## Result shape

```actus
open verb load(abs path: Path) -> Result[Buffer, IoError] {
    erg bytes = read_to_bytes(path: abs path)?;
    return Result[Buffer, IoError].Ok(bytes);
}
```

`Ok` owns or borrows exactly the value promised by the operation. `Err` owns an
error enum whose domain belongs to the module. Do not replace an error with a
zero, empty buffer, false flag, or generic success.

## Branching

Use `case dat value` when inspecting or translating a result:

```actus
return case dat decoded {
    Result.Ok(frame) => inspect(frame: abs frame),
    Result.Err(error) => report(error: dat error),
};
```

The `dat` subject makes ownership consumption explicit. Every branch must
finish the payload's ownership path by returning, translating, dropping, or
passing it to another owner.

Use `?` when the enclosing verb returns a compatible `Result` type. Use an
explicit `case` when a branch needs to distinguish variants or change the
success type.

## Error domains

| Domain | Used for |
| --- | --- |
| `IoError` | console, buffers, files, streams, end-of-stream, invalid data |
| `PathError` | path representation, roots, components, capacity, encoding |
| `StringError` | borrowed text, UTF-8 validation, byte bounds, storage |
| `TimeError` | monotonic arithmetic, precision, provider and timer state |
| `RegionError` | region descriptor, generation, window and lifecycle checks |
| `WireError` | framing, CRC, sequence, fragment and reassembly contracts |
| `WireEndpointError` | optional bounded endpoint syntax |

The caller should preserve the most specific domain until it reaches an
intentional application boundary.

## Raw provider status

Private runtime bridges may use signed integers for ABI compatibility. Public
facades translate those values immediately into typed errors. Application code
must never compare `-1`, `-2`, or another bridge code directly.

## Side effects and errors

An error does not undo a side effect that already happened. A successful file
write followed by a failed flush is still a failed publication lifecycle. A
successful timer poll may already have delivered an event when a later
reschedule fails. Read each module's cleanup and commit contract before
choosing recovery behavior.

## Ownership checklist

Before returning an error, confirm:

1. temporary buffers and handles were dropped or returned;
2. moved error payloads were consumed exactly once;
3. no partial output was published as successful;
4. the original failure was preserved when cleanup also failed;
5. the caller can distinguish retryable, invalid, stale, and unavailable cases.

## Detailed pages

- [Error ownership](ownership.md)
- [Usage patterns](usage.md)
- [Complete standard-library reference](../reference/20-standard-library.md)
- [Language ownership rules](../../ownership/case-and-branch-ownership.md)
