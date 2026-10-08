# `std::string`

`std::string` is the standard library boundary for inspecting borrowed
`String` values and for building a validated, caller-buffer-backed UTF-8
value. It does not hide allocation, expose raw pointers, decode Unicode for
the caller, or silently repair malformed bytes.

```actus
import std::string;
```

## What this library provides

The facade exposes two related representations:

- `String` is the runtime-provided, borrowed text value. Access functions
  inspect it through `abs`, so the caller keeps ownership and the operation
  cannot mutate it.
- `Utf8Buffer` is an owned aggregate containing a caller-provided `Buffer`
  and its validated byte length. Construction takes the buffer with `dat` and
  returns it only as part of the successful value.

The public API is byte-oriented. A length is a count of UTF-8 bytes and an
index selects one byte, not one Unicode scalar or grapheme. A multibyte code
point therefore occupies several consecutive indexes.

## Reading order

1. [API](api.md) — complete public declarations and result contracts.
2. [Representations](representation.md) — `String`, `Buffer`, and
   `Utf8Buffer` boundaries.
3. [Ownership](ownership.md) — `abs`, `dat`, `ins`, cleanup, and capacity.
4. [Encoding and errors](encoding.md) — UTF-8 validation and error handling.
5. [Errors](errors.md) — meaning of every public failure variant.
6. [Runtime behavior](runtime.md) — hosted provider requirements and limits.
7. [Usage](usage.md) and [examples](examples.md) — common call patterns.

## Basic choice

Use `string_length` or `string_byte_at` when a borrowed runtime `String` is
already available. Use `utf8_from_buffer` when bytes already live in a
`Buffer`. Use `utf8_from_string` when a borrowed `String` must be copied into
caller-provided storage and retained as a validated value.

The destination capacity must be chosen by the caller. If it is too small,
construction returns `StringError.CapacityExceeded`; the API does not allocate
a larger buffer behind the caller's back.

## Related contracts

- [`std::io`](../io/README.md) defines text and byte output operations.
- [`std::filesystem`](../filesystem/README.md) defines file operations that
  may receive or produce runtime strings.
- The canonical implementation is in
  [`library/std/src/string/`](../../../../library/std/src/string/).
- Native and facade tests are in
  [`tests/std_string_native.rs`](../../../../tests/std_string_native.rs) and
  [`tests/std_string_semantic.rs`](../../../../tests/std_string_semantic.rs).

## Current target boundary

The current facade depends on a hosted string provider. A freestanding target
may use the same public API only after it supplies an equivalent checked
provider contract. The public contract remains the same: borrowed inspection,
explicit caller storage, typed failure, and no hidden allocation.
