# `std::io`

```actus
import std::io;
```
`std::io` provides console streams, reader and writer contracts, buffered
operations, in-memory cursors, and typed IO errors.

The facade exports `stdin`, `stdout`, `stderr`, read/write abstractions,
reader/writer performances, cursor streams, copying, and buffered helpers.

Use caller-owned buffers for streaming input and output. `ins Buffer` is used
when an operation fills or mutates existing storage; `abs Buffer` is used for
read-only output or inspection. Public operations return typed results where
input, output, or runtime IO can fail.

Raw runtime bridges stay private to the facade. Application code should not
import or call them directly.

## Public surface

The facade includes:

- `read_line(ins buffer: Buffer)` and `read_byte()` for stdin;
- `print`, `println`, `print_int`, `printb`, `printlnb`, and `flush` for stdout;
- `eprint_int`, `eprint`, and `eprintln` for stderr;
- `read` and `write` reader/writer contracts;
- `Cursor` and cursor read, write, seek, and flush operations;
- buffered reader and writer constructors, refill, reserve, append, clear, and
  flush operations;
- generic `copy_stream` for compatible reader and writer performances.

Read operations fill caller-owned `ins Buffer` storage and return a typed
`IoError` result. Output operations inspect `abs` data and return the number of
processed bytes or a typed error. Constructors that take a resource use the
role shown by their declaration and clean up the owned value according to the
stream contract.
