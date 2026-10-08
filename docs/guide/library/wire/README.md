# `std::wire`

`std::wire` is a bounded, transport-neutral binary framing library. It
encodes and validates fixed-width metadata, payloads, CRC16 integrity trailers,
incremental input, replay-window state, fragmentation, and optional endpoint
text parsing.

```actus
import std::wire;
```

The core protocol operates on caller-owned `Buffer` values. It does not open
sockets, access a filesystem, allocate a heap buffer, select a transport, or
require a `wire://` string. A desktop application may use the optional
endpoint parser as a convenient address representation; embedded code may
exchange frames using numeric identifiers and byte buffers only.

## Reading order

1. [API](api.md) — public types, constants, verbs, and result mapping.
2. [Frame protocol](protocol.md) — header layout, payload, CRC, and flags.
3. [Codec and parser](codec-and-parser.md) — complete frames and split input.
4. [Sequence window](sequence.md) — duplicate, stale, and context handling.
5. [Fragments](fragments.md) — bounded message reassembly.
6. [Endpoints](endpoints.md) — optional `wire://` parser and offset views.
7. [Ownership and lifecycle](lifecycle.md), [errors](errors.md), and
   [runtime](runtime.md).
8. [Usage](usage.md) and [examples](examples.md).

## Core versus optional layers

The core layers are frame metadata, CRC, encode/decode, incremental parsing,
sequence acceptance, and fragment reassembly. They are usable with a serial
link, shared memory, a file-like test fixture, a device driver, or another
transport selected by the caller.

`wire_endpoint_parse` is an optional hosted convenience. It recognizes a
bounded `wire://authority[:port][/path]` byte form and returns offsets into the
unchanged input. It does not open a connection or make endpoint syntax
mandatory for frame exchange.

## Security boundary

CRC16 detects accidental corruption. It is not cryptographic authentication,
anti-tamper protection, authorization, or encryption. A higher-level protocol
must add those properties explicitly when required.

## Source and evidence

The implementation is in
[`library/std/src/wire/`](../../../../library/std/src/wire/). Native coverage
is in [`tests/std_wire_native.rs`](../../../../tests/std_wire_native.rs); facade
and visibility coverage is in [`tests/std_wire_semantic.rs`](../../../../tests/std_wire_semantic.rs).
