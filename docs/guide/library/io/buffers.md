# Buffers, capacity, and buffered adapters

`std::io` separates a buffer's logical length from its reserved capacity.
Reading and writing use existing caller-owned storage; the library does not
turn every operation into an implicit growing collection.

## Logical length and capacity

- `buffer_size(abs buffer)` returns initialized logical bytes.
- `reserve(ins buffer, capacity)` increases available capacity without
  changing logical length.
- `buffer_clear(ins buffer)` sets logical length to zero while retaining the
  allocation.
- `buffer_append_range(ins target, abs source, offset, length)` appends a
  bounded source range and returns the exact count accepted.

A capacity failure or rejected handle is returned as `IoError.Failed`. A zero
append result is represented as `IoError.InvalidData` by the public wrapper;
the caller must not interpret it as a successful transfer of bytes.

## Read destinations

`read_line`, `read`, `cursor_read`, and a `Reader` implementation receive an
`ins Buffer`. The operation may fill or mutate the existing storage, while the
caller remains the owner after the loan ends. The returned count is the exact
number of bytes written.

The caller chooses enough capacity for the expected operation. A read does not
return a replacement buffer through a hidden allocation.

## Write sources

`write`, `printb`, `eprint`, `cursor_write`, and a `Writer` implementation
receive an `abs Buffer`. The source is inspected but not consumed. A writer
may accept fewer bytes than requested, so code that requires complete delivery
must compare the returned count with the requested length.

## `BufferedReader`

```actus
struct BufferedReader[Source: Reader] {
    source: Source,
    buffer: Buffer,
}
```

`buffered_reader(dat source, dat buffer)` transfers both values into one
adapter. `refill(ins reader)` fills the adapter's reusable buffer from its
statically selected source. The buffer remains available for inspection after
the call.

## `BufferedWriter`

```actus
struct BufferedWriter[Target: Writer] {
    target: Target,
    buffer: Buffer,
}
```

`buffered_writer(dat target, dat buffer)` transfers both values into the
adapter. `buffered_write` appends input into pending storage and flushes when
that storage is full. `flush_buffer` writes pending bytes, clears logical
length after a successful target write, and retains capacity for reuse.

A final explicit flush is required for a partial pending buffer. Returning
from `buffered_write` alone does not guarantee that every pending byte reached
the target.

## Copy scratch storage

`copy_stream` reserves one 4096-byte scratch buffer for the operation. It
reuses that buffer for every read, rejects a short write, and treats
`EndOfStream` as successful completion with the accumulated count. The scratch
buffer is cleaned up when the operation returns.
