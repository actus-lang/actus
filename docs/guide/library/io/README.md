# `std::io`

`std::io` is Actus's typed interface for console input and output and for
stream-shaped components. It contains five related parts:

| Part | Use it for |
| --- | --- |
| Console input | Reading a line or one byte from standard input. |
| Console output | Writing strings, bytes, integers, and flushing stdout. |
| `Reader` and `Writer` | Giving different stream types one common contract. |
| `Cursor` | Reading and writing an in-memory `Buffer` as a stream. |
| Buffered adapters | Reusing a caller-owned buffer around a reader or writer. |
| `copy_stream` | Copying data between compatible reader and writer values. |

```actus
import std::io;

verb main() -> Int {
    erg message = Buffer[0];
    append(message, 72u8);
    append(message, 105u8);
    printb(text: abs message)?;
    flush()?;
    return 0;
}
```

`std::io` does not hide ownership. A buffer used as an input or output
destination is passed with `ins`; a buffer that is only inspected is passed
with `abs`. The library returns `Result[Int, IoError]` for operations that can
fail.

## Module files

The public facade is `library/std/src/io/io.act`. It re-exports declarations
from these implementation units:

- `stdin.act`: `read_line`, `read_byte`, and the `read` reader operation;
- `stdout.act`: string, byte, integer, and flush operations;
- `stderr.act`: error-stream output operations;
- `reader.act` and `writer.act`: the static `Reader` and `Writer` contracts;
- `cursor.act`: the in-memory `Cursor` type and its operations;
- `buffered.act`: buffered adapters and buffer helpers;
- `copy.act`: generic stream copying;
- `error.act`: `IoError`.

## Reading order

1. [API reference](api.md)
2. [Ownership and errors](ownership-and-errors.md)
3. [Examples](examples.md)
4. [Runtime and performance boundaries](runtime.md)

## What `std::io` does not do

The module does not provide an asynchronous runtime, a scheduler, a network
transport, or an unbounded stream abstraction. It works with caller-owned
bounded buffers and the runtime services supplied by the selected profile.
