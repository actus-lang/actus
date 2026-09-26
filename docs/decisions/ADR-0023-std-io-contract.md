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
open verb print_int(erg value: Int) -> Int;
open verb eprint_int(erg value: Int) -> Int;
```

Both operations return the printed scalar value. They do not transfer or
allocate an Actus resource. The stdout implementation calls
`actus_print_int`; the stderr implementation calls
`actus_print_int_stderr`. Those runtime declarations cross an explicit
`unsafe extern "C"` boundary and are not compiler intrinsics.

Gate 2.1 extends the output API with borrowed, length-aware buffer
operations:

```act
open verb print(abs text: Buffer) -> Int;
open verb println(abs text: Buffer) -> Int;
open verb eprint(abs text: Buffer) -> Int;
open verb eprintln(abs text: Buffer) -> Int;
open verb flush() -> Int;
```

The buffer operations never consume, mutate, allocate, or drop the source
buffer. Their runtime bridges receive the existing buffer handle and write its
declared byte range. `println` and `eprintln` append one newline; `flush`
flushes stdout explicitly. A successful output returns the byte count, while
runtime failure returns `-1`.

The runtime prints one newline per call. Output ordering between stdout and
stderr is determined by the host streams; each stream's bytes are otherwise
deterministic for a successful call. Input, general stream abstractions,
buffering, and cursor utilities remain separate Gate 2 increments.

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
