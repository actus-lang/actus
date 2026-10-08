# `std::io` API reference

This page explains the public operations exported by `import std::io;`. The
signatures below are grouped by the job they perform. Raw `unsafe extern "C"`
bridges are intentionally omitted because application code uses the typed
Actus wrappers.

## `IoError`

`IoError` has four variants:

| Variant | Meaning |
| --- | --- |
| `Failed` | The host or target rejected the operation. |
| `EndOfStream` | No more input is available. |
| `InvalidInput` | A buffer, stream, or argument is invalid. |
| `InvalidData` | The data or requested range is not accepted. |

## Console input

| Verb | Contract |
| --- | --- |
| `read_line(ins buffer: Buffer) -> Result[Int, IoError]` | Fills the existing buffer through the next newline or EOF. |
| `read_byte() -> Result[Int, IoError]` | Returns one byte as `Int`, or a typed failure. |
| `read(ins buffer: Buffer) -> Result[Int, IoError]` | Reader-shaped alias for `read_line`. |

`read_line` does not create a replacement buffer. The caller must provide
storage with sufficient capacity and retain ownership after the call.

## Console output

| Verb | Contract |
| --- | --- |
| `print(abs text: String) -> Result[Int, IoError]` | Writes a borrowed `String` without a newline. |
| `println(abs text: String) -> Result[Int, IoError]` | Writes a borrowed `String` and a newline. |
| `print_int(erg value: Int) -> Result[Int, IoError]` | Writes one integer value. |
| `printb(abs text: Buffer) -> Result[Int, IoError]` | Writes borrowed bytes without a newline. |
| `printlnb(abs text: Buffer) -> Result[Int, IoError]` | Writes borrowed bytes, newline, and flushes. |
| `flush() -> Result[Int, IoError]` | Flushes stdout and returns `Ok(0)` on success. |
| `write(abs buffer: Buffer) -> Result[Int, IoError]` | Writes borrowed bytes and returns the exact byte count. |

Use `print` for the language's `String` value and `printb` for raw bytes. A
`Buffer` is not silently interpreted as text.

## Stderr

| Verb | Contract |
| --- | --- |
| `eprint_int(erg value: Int) -> Result[Int, IoError]` | Writes an integer to stderr. |
| `eprint(abs text: Buffer) -> Result[Int, IoError]` | Writes borrowed bytes to stderr. |
| `eprintln(abs text: Buffer) -> Result[Int, IoError]` | Writes bytes, newline, and flushes stderr. |

## Static stream contracts

`Reader` and `Writer` are Actus roles used for static generic dispatch.

```actus
open role Reader {
    verb read(ins self: Self, ins buffer: Buffer) -> Result[Int, IoError];
}

open role Writer {
    verb write(ins self: Self, abs buffer: Buffer) -> Result[Int, IoError];
    verb flush(ins self: Self) -> Result[Int, IoError];
}
```

A reader fills a destination buffer. A writer observes source bytes and may
mutate its own stream state. A writer must report how many bytes it accepted;
the caller can reject a short write as an I/O failure.

## `Cursor`

`Cursor` is an in-memory stream:

```actus
open struct Cursor {
    erg buffer: Buffer,
    erg position: Int,
}

open verb cursor(dat buffer: Buffer) -> Cursor;
open verb cursor_read(ins self: Cursor, ins buffer: Buffer) -> Result[Int, IoError];
open verb cursor_write(ins self: Cursor, abs buffer: Buffer) -> Result[Int, IoError];
open verb seek(ins self: Cursor, erg offset: Int) -> Result[Int, IoError];
open verb cursor_flush(ins self: Cursor) -> Result[Int, IoError];
```

`Result[Int, IoError]` in source. `cursor` consumes the backing buffer. Reads
and writes advance `position`; `seek` changes it only when the requested
position is valid. Reading at the end returns `EndOfStream`.

## Buffered adapters

```actus
open struct BufferedReader[Source: Reader] {
    erg source: Source,
    erg buffer: Buffer,
}

open struct BufferedWriter[Target: Writer] {
    erg target: Target,
    erg buffer: Buffer,
}
```

Public operations:

| Verb | Behavior |
| --- | --- |
| `buffered_reader(dat source, dat buffer)` | Transfers both values into a reader adapter. |
| `buffered_writer(dat target, dat buffer)` | Transfers both values into a writer adapter. |
| `refill(ins reader)` | Reads into the reader's existing buffer. |
| `buffer_size(abs buffer)` | Returns logical length. |
| `reserve(ins buffer, erg capacity)` | Increases reusable capacity without changing logical length. |
| `buffer_append_range(ins target, abs source, erg offset, erg length)` | Appends a bounded range. |
| `buffer_clear(ins buffer)` | Clears logical length and retains capacity. |
| `buffered_write(ins writer, abs input)` | Appends input and flushes when the buffer is full. |
| `flush_buffer(ins writer)` | Writes pending bytes and clears logical length. |

The constructors use `dat` because the adapter owns the source/target and
storage. The adapter is cleaned up as one aggregate.

## `copy_stream`

```actus
open verb copy_stream[R: Reader, W: Writer](
    ins reader: R,
    ins writer: W
) -> Result[Int, IoError];
```

The concrete reader and writer are selected statically. The implementation
uses one reusable 4096-byte scratch buffer, repeatedly reads a chunk, writes
the complete chunk, and accumulates the byte count. `EndOfStream` is the
successful termination condition. A short write, invalid zero-byte read, or
other error is returned as `IoError`.
