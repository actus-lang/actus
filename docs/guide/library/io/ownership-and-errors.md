# `std::io`: ownership, failure, and cleanup

## Buffer roles

| Role | Meaning in I/O |
| --- | --- |
| `abs Buffer` | The operation may inspect bytes but does not consume or mutate the buffer. |
| `ins Buffer` | The operation may fill, append to, clear, or otherwise mutate caller-owned storage for the call. |
| `dat Buffer` | The operation takes ownership and becomes responsible for cleanup. |
| `ins Cursor` | The cursor's position and backing storage may change during the call. |

The role must be visible at the declaration and at every call site:

```actus
erg input = Buffer[0];
erg output = Buffer[0];

read_line(buffer: ins input)?;
printb(text: abs input)?;

erg source = Cursor { buffer: input, position: 0, };
erg target = Cursor { buffer: output, position: 0, };
copy_stream(reader: ins source, writer: ins target)?;
```

The cursor construction consumes the two buffers. The original buffer owners
must not be used after that transfer unless a valid returned owner restores
them.

## Result handling

Every fallible public operation returns `Result[Int, IoError]` or another
typed result. Handle it explicitly:

```actus
erg result = read_line(buffer: ins input);
return case dat result {
    Result.Ok(count) => count,
    Result.Err(error) => case dat error {
        IoError.EndOfStream => 0,
        IoError.Failed => 1,
        IoError.InvalidInput => 2,
        IoError.InvalidData => 3,
    },
};
```

Use `?` when the surrounding verb returns a compatible `Result`. The raw
negative statuses produced by runtime bridges never belong in application
code.

## Short reads and short writes

`Ok(count)` reports the exact number of bytes processed. It does not mean that
the requested range was necessarily completed. A caller that requires the
whole range must compare `count` with the requested length. `copy_stream`
performs this check internally and returns `IoError.Failed` for a short write.

## Buffered writer lifecycle

`buffered_write` can leave bytes pending in the adapter. The normal lifecycle
is:

1. Construct the adapter with owned target and scratch buffer.
2. Call `buffered_write` for one or more inputs.
3. Call `flush_buffer` before handing the writer to another component or
   leaving the scope.
4. Let scope cleanup release the adapter's owned fields.

The backing buffer is cleared after a successful flush, but its capacity is
retained for the next write.

## Runtime boundary

Console I/O and the raw buffer bridges require a runtime provider. The typed
Actus facade translates the provider status into `IoError`. A freestanding
target must supply the corresponding provider before these operations can be
used; the language type checker does not turn an absent device into a valid
stream.
