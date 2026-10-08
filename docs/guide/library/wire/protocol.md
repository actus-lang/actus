# Frame protocol

## Complete frame shape

A version-one frame is:

```text
+----------------------+  12 bytes
| fixed WireHeader     |
+----------------------+
| payload              |  0..1024 bytes
+----------------------+
| CRC16-CCITT          |  2 bytes, little-endian
+----------------------+
```

The maximum complete frame is 1038 bytes. The header's `payload_len` must
match the payload supplied to the encoder and is checked before any output is
published.

## Header fields

| Wire offset | Field | Size | Meaning |
| ---: | --- | ---: | --- |
| 0 | `magic_0` | 1 | `0xDA` |
| 1 | `magic_1` | 1 | `0x11` |
| 2 | `version` | 1 | version `1` |
| 3 | `flags` | 1 | known protocol flags only |
| 4 | `message_id` | 1 | application-neutral message selector |
| 5 | `channel_id` | 1 | application-neutral channel selector |
| 6..9 | `sequence_num` | 4 | little-endian sequence number |
| 10..11 | `payload_len` | 2 | little-endian payload byte count |

The header stores metadata only. It does not own the payload or identify a
transport endpoint.

## Flags

The known flags are `ACK_REQUIRED` (`0x02`), `URGENT` (`0x04`), and
`COMPRESSED` (`0x08`). Unknown bits are rejected with `UnknownFlags`. A flag
bit describes a higher-level policy; the codec does not implement compression,
acknowledgement delivery, prioritization, or encryption.

## CRC16-CCITT

The checksum uses polynomial `0x1021`, initial register `0xFFFF`, no reflection,
and zero final XOR. It covers the header and payload, excluding the two-byte
trailer. The encoded checksum is little-endian.

CRC detects accidental changes and truncation. It does not authenticate a
sender or protect against intentional modification.

## Validation order

A decoder must establish enough bytes for the header, validate magic and
version, reject unknown flags, enforce payload maximum, establish the complete
frame length, validate the checksum, and only then copy payload to caller
storage. A failure must not publish partial payload state.

## Transport neutrality

The frame is a byte contract. The caller chooses how bytes move: a serial
adapter, shared memory, a desktop stream, a device queue, or a test buffer.
`std::wire` itself does not select a port, path, socket, or URL.
