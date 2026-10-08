# `std::io` examples

## Print text bytes

```actus
import std::io;

verb main() -> Int {
    erg text = Buffer[0];
    append(text, 72u8);
    append(text, 105u8);
    printb(text: abs text)?;
    printlnb(text: abs text)?;
    return 0;
}
```

`append` grows the caller's buffer according to its configured capacity. The
I/O verbs only inspect it through `abs`.

## Read a line into caller storage

```actus
import std::io;

verb read_once(ins destination: Buffer) -> Result[Int, IoError] {
    return read_line(buffer: ins destination);
}
```

The caller owns `destination` before and after the call. The function does not
return a new buffer.

## Use a cursor as both reader and writer

```actus
import std::io;

verb round_trip() -> Result[Int, IoError] {
    erg input = Buffer[0];
    append(input, 65u8);
    append(input, 66u8);

    erg source = Cursor { buffer: input, position: 0, };
    erg output = Buffer[0];
    erg target = Cursor { buffer: output, position: 0, };

    copy_stream(reader: ins source, writer: ins target)?;
    seek(self: ins target, offset: 0)?;

    erg result = Buffer[0];
    return cursor_read(self: ins target, buffer: ins result);
}
```

Both cursors own their buffers. `copy_stream` uses the static `Reader` and
`Writer` performances implemented for `Cursor`.

## Buffered output

```actus
import std::io;

verb buffered_output(ins target: Cursor, abs payload: Buffer) -> Result[Int, IoError] {
    erg scratch = Buffer[0];
    reserve(buffer: ins scratch, capacity: 64)?;
    erg writer: BufferedWriter[Cursor] = BufferedWriter[Cursor] {
        target: target,
        buffer: scratch,
    };
    buffered_write(writer: ins writer, input: abs payload)?;
    return flush_buffer(writer: ins writer);
}
```

`buffered_write` may flush automatically when the scratch buffer is full. The
explicit final `flush_buffer` handles the remaining partial chunk.

## Implement a custom reader contract

```actus
import std::io;

struct FixedReader {
    erg marker: Int,
}

perform Reader for FixedReader {
    verb read(ins self: FixedReader, ins buffer: Buffer) -> Result[Int, IoError] {
        return Result[Int, IoError].Err(IoError.EndOfStream);
    }
}
```

The example shows the declaration shape and an explicit end-of-stream result.
A real reader replaces the body with its device or storage operation and must
preserve its position in its own state. A `perform Reader` declaration must
satisfy the exact role and return contract of the public `Reader` role.
