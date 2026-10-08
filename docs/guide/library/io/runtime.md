# `std::io`: runtime and performance boundaries

## Hosted services

`stdin`, `stdout`, `stderr`, and the raw buffer operations call runtime
providers. The public facade keeps those bridges private and converts their
status codes into typed Actus results. Hosted applications can use the
console operations after selecting the hosted runtime profile.

## Bounded storage

The library does not provide an unbounded stream object. `Buffer` storage is
caller-controlled. `reserve` changes capacity, while `buffer_clear` changes
logical length and keeps the allocation available for reuse.

`copy_stream` uses a fixed 4096-byte scratch buffer for each invocation. The
buffer is released by normal scope cleanup when the operation returns. It does
not retain input bytes after the call.

## Blocking behavior

Console reads, writes, and flushes may block according to the selected runtime
provider. They should not be placed in a latency-sensitive computation path
unless that blocking boundary is intentional. `Cursor` operations are
in-memory and do not contact the operating system.

## Raw bridges

Declarations such as `actus_read_stdin_line`, `actus_write_buffer_stdout`, and
`actus_buffer_reserve` are private implementation bridges. Application code
must use `read_line`, `write`, `reserve`, and the other typed public verbs.
This keeps runtime status encoding and buffer-handle validation outside the
application API.

## Validation

The repository tests cover public visibility, ownership roles, console output,
line input, cursor reads and writes, buffered adapters, stream copying, short
writes, and runtime failure translation. When changing `std::io`, run the
focused standard-library tests before the complete Actus test suite.
