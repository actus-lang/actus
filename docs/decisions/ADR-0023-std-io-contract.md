# ADR-0023: `std::io` Console Output Contract

- Status: Accepted
- Date: 2026-09-26
- Scope: Phase 16 Gate 2

## Decision

The first standard-library module is a directory package under
`library/std`. Its public path is built from facade files:

```text
library/std/src/lib.act -> io/io.act -> stdout.act, stderr.act
```

Each facade uses `open` to expose the next layer. Implementation files keep
the runtime bridge declaration next to the Actus-facing wrapper, while the
package facade exposes only the intended public verbs.

The initial output API is:

```act
open verb print_int(erg value: Int) -> Result[Int, IoError];
open verb eprint_int(erg value: Int) -> Result[Int, IoError];
```

Both operations report a typed result. A successful call reports the runtime
status value in `Ok`; a runtime failure is represented by `Err(Failed)`. They
do not transfer or allocate an Actus resource. The stdout implementation calls
`actus_print_int`; the stderr implementation calls
`actus_print_int_stderr`. Those runtime declarations cross an explicit
`unsafe extern "C"` boundary and are not compiler intrinsics.

Gate 2.1 extends the output API with borrowed, length-aware buffer
operations:

```act
open verb print(abs text: Buffer) -> Result[Int, IoError];
open verb println(abs text: Buffer) -> Result[Int, IoError];
open verb eprint(abs text: Buffer) -> Result[Int, IoError];
open verb eprintln(abs text: Buffer) -> Result[Int, IoError];
open verb flush() -> Result[Int, IoError];
```

The buffer operations never consume, mutate, allocate, or drop the source
buffer. Their runtime bridges receive the existing buffer handle and write its
declared byte range. `println` and `eprintln` append one newline; `flush`
flushes stdout explicitly. A successful output returns the byte count, while
runtime failure returns `-1`.

The runtime prints one newline per call. `println` and `eprintln` flush their
corresponding stream before returning. Output ordering between stdout and
stderr is determined by the host streams; each stream's bytes are otherwise
deterministic for a successful call. Input, general stream abstractions,
buffering, and cursor utilities remain separate Gate 2 increments.

Gate 2.2 adds stdin operations:

```act
open verb read_line(ins buffer: Buffer) -> Result[Int, IoError];
open verb read_byte() -> Result[Int, IoError];
```

`read_line` consumes input through the next newline or EOF, excludes the
newline, and appends bytes into the caller-owned buffer under an `ins` loan.
It returns the byte count, `-2` for immediate EOF, or `-1` for an input
failure. `read_byte` returns an unsigned byte value, `-2` at EOF, or `-1` on
failure. The runtime restores the caller's owner after `read_line` returns.

## Ownership and target behavior

The scalar output operations use `erg` parameters and therefore do not create
borrow or transfer obligations. The buffer output operations use `abs` and
therefore preserve the caller's owner. The public Actus API does not expose
Rust implementation types.

`std::io` is available only when the selected target provides the hosted
runtime capability. A freestanding target may omit the runtime archive and
must not silently acquire host console services.

## Verification

The package facade and sibling exports are resolved and semantically checked.
A native integration test imports the `io` facade, links the hosted runtime,
and verifies independent stdout and stderr output with exit status zero.
