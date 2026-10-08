# `std::wire`

```actus
import std::wire;
```
`std::wire` is a target-neutral bounded binary framing library. It provides
fixed-width frame metadata, typed protocol errors, checksum calculation,
encoding and decoding, incremental parsing, sequence-window replay
suppression, fragmentation, and caller-owned reassembly.

The initial frame contract uses a 12-byte header, a bounded payload, and a
CRC16 trailer. All multi-byte fields use the declared little-endian layout.
CRC detects accidental corruption; it is not authentication or authorization.

The core does not require a URI, operating system, filesystem, transport,
allocation, or cryptographic service. `wire://` endpoint parsing is optional
and belongs to a hosted endpoint layer. Direct typed frame APIs and numeric
channels remain valid communication paths.

See the protocol ADR and the Wire examples for complete frame fields and
lifecycle operations.

## Public surface

The facade includes:

- `WireHeader` and version-one frame constants;
- `WireError` typed protocol failures;
- header and complete-frame encode/decode operations;
- `wire_crc16_ccitt` over a bounded input range;
- `WireParser` construction, reset, status, consumed-byte, header, and feed
  operations;
- `WireSequenceWindow` construction, reset, context replacement, highest
  sequence, and bounded admission operations;
- `WireFragmentHeader`, bounded fragment encode/decode, and
  `WireReassembly` open, begin, accept, copy, and cancel operations;
- optional hosted `wire://` endpoint parsing with bounded fixed metadata.

Frame and parser inputs are read through `abs Buffer`; output and reassembly
storage use caller-owned `ins Buffer` according to the declaration. The core
has no hidden allocation or synchronous transport I/O.

## Detailed pages

- [Protocol reference](protocol.md)
- [Protocol reference](../reference/20-standard-library.md)
- [Wire implementation](../../../../library/std/src/wire/)
- [Native Wire tests](../../../../tests/std_wire_native.rs)
- [Semantic Wire tests](../../../../tests/std_wire_semantic.rs)

## Processing lifecycle

1. Build a `WireHeader` with a supported version, flags, channel, sequence,
   and payload length.
2. Encode the header or complete frame into caller-owned output storage.
3. On receive, feed arbitrary byte chunks to `WireParser`; a split chunk is a
   normal input, not an error.
4. Read the validated header and copied payload only after the parser reaches
   `Ready`.
5. Use `WireSequenceWindow` to reject duplicates, stale sequence numbers, and
   context mismatches.
6. For messages larger than one frame, encode bounded fragments and use
   `WireReassembly` with an explicit generation, capacity, and cancellation
   path.

`wire://` endpoint parsing is optional hosted metadata. The binary frame API
does not require a URI, string, socket, or transport implementation.
