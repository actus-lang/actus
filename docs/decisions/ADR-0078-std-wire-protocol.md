# ADR-0078: Actus `std::wire` Universal Binary Communication Protocol

- Status: Proposed
- Date: 2026-10-07
- Scope: Actus standard-library communication and bounded binary framing
- Supersedes: ADR-0051

## Context

Actus needs one target-neutral communication foundation that can carry ordinary
application data, telemetry, commands, model events, and device messages.
The library must be usable by hosted programs and freestanding targets without
depending on an operating system, board, filesystem, UI, or downstream
application.

The former design separated a generic `std::wire` codec from a second
`std::ustari` protocol. That split is unnecessary for the current direction.
The standard-library contract is therefore consolidated under `std::wire`.
`wire` is the protocol name and the only public namespace introduced by this
ADR. No application or device terminology belongs in the core library.

## Decision

Actus will provide a target-neutral `std::wire` library with a bounded,
deterministic frame codec. Physical transports and application schemas remain
separate adapters and profiles.

### 1. Version 1 frame

Each frame consists of a fixed 12-byte header, a bounded payload, and a
2-byte CRC trailer:

```text
offset  size  field
0x00      2   magic bytes: 0xDA, 0x11
0x02      1   version: 1
0x03      1   flags
0x04      1   message ID
0x05      1   channel ID
0x06      4   sequence number, u32 little-endian
0x0A      2   payload length, u16 little-endian
0x0C      N   payload, 0..=1024 bytes
0x0C+N    2   CRC16-CCITT, little-endian
```

The version 1 constants are:

- `WIRE_VERSION = 1`;
- `WIRE_HEADER_BYTES = 12`;
- `WIRE_CRC_BYTES = 2`;
- `WIRE_MAX_PAYLOAD_BYTES = 1024`;
- `WIRE_MAX_FRAME_BYTES = 1038`;
- all multi-byte fields are little-endian;
- CRC polynomial `0x1021`, initial value `0xFFFF`, no reflection, and final
  XOR `0x0000`.

Magic is a byte sequence and must not depend on host endianness. A later
incompatible frame layout requires a new protocol version.

### 2. Flags and generic channels

The core defines named flag values but does not assign application meaning to
payloads:

- `ACK_REQUIRED = 0x02`;
- `URGENT = 0x04`;
- `COMPRESSED = 0x08`, only when a bounded codec is explicitly negotiated;
- `ENCRYPTED = 0x01` is reserved for a future security profile and is rejected
  by the initial non-cryptographic implementation.

Unknown flag bits are rejected with a typed error. Initial generic channels
are control, telemetry, and application data. Their numeric values are part of
the versioned contract; application-specific schemas must not redefine them.

The initial implementation does not provide encryption. CRC detects accidental
corruption only and is never treated as authentication or authorization.

### 3. Actus ownership and allocation contract

The core uses caller-owned bounded storage:

- `abs Buffer` for read-only frame and payload inspection;
- `ins Buffer` for parser, output, and reassembly storage;
- `erg` for owned parser or reassembly state;
- `dat` only where an operation intentionally transfers terminal ownership.

Complete-frame parsing should expose a zero-copy payload view when the caller
owns the input buffer. The core must not require a heap allocator, hidden
filesystem access, raw operating-system pointers, or unbounded storage.

### 4. Codec and parser behavior

The public API must provide responsibility-oriented operations for:

- fixed-width header encoding and decoding;
- frame serialization with output-capacity validation;
- CRC calculation and verification;
- incremental parsing from arbitrary byte chunks;
- consumed-byte and incomplete-frame reporting;
- bounded resynchronization after bad magic, version, flags, length, or CRC;
- typed errors for framing, capacity, integrity, version, and protocol policy.

Length and capacity checks occur before payload access. Malformed input must not
panic, access outside caller storage, loop indefinitely, or silently become a
valid message.

### 5. Sequence handling

Every frame carries a sequence number scoped to a communication direction and
session or endpoint context. The initial library provides bounded duplicate and
stale-frame detection using a configurable sliding window. This prevents
accidental replay within the active context; it is not authentication without
an authenticated session.

Sequence wrap, reset, and context replacement must be explicit. A receiver
must reject duplicates, stale values, and unreasonable forward jumps according
to the selected bounded profile.

### 6. Fragmentation and transports

The core frame codec operates on complete frames. Transport adapters own byte
stream behavior, MTU, packet boundaries, retry, timeout, and connection state.

Adapters may include TCP, Serial, USB, CAN, WebSocket, or other transports,
but none is part of the target-neutral core. Fragmentation and reassembly are
separate bounded primitives with explicit limits for fragments, offsets,
timeouts, duplicates, cancellation, and resource exhaustion.

### 7. `wire://` addressing

`wire://` is an endpoint-addressing convention, not a replacement for the
binary frame codec. A future endpoint module may parse addresses such as
`wire://localhost/control`, while transport adapters decide how to connect.
Address parsing must not introduce filesystem, socket, or OS dependencies into
the core codec.

### 8. Application schemas

The universal core carries bounded bytes and generic message metadata. A
consumer defines its own versioned payload schema, maximum size, ownership
contract, error behavior, and compatibility policy. The core must not contain
neural-engine, device, robotics, or UI-specific message definitions.

## Module boundary

```text
library/std/src/wire/wire.act          # canonical public facade
library/std/src/wire/frame.act         # frame model and constants
library/std/src/wire/codec.act         # serialization and decoding
library/std/src/wire/parser.act        # incremental parser state
library/std/src/wire/checksum.act      # CRC16-CCITT
library/std/src/wire/sequence.act      # bounded sequence window
library/std/src/wire/fragment.act      # bounded reassembly
library/std/src/wire/error.act         # typed errors
```

Every public declaration must be exported through the canonical facade. Files
must remain responsibility-oriented and below the Actus source-size limits.

## Invariants

1. `std::wire` has no dependency on a specific OS, board, filesystem,
   transport, UI, or application.
2. Every frame has a bounded length, explicit version, and deterministic byte
   order.
3. Payload and reassembly bounds are checked before storage access.
4. Unknown versions and flag bits fail closed with typed errors.
5. CRC is integrity detection only.
6. The initial implementation has no encryption or crypto primitive.
7. The parser and serializer perform no hidden allocation or unbounded work.
8. Transport adapters cannot alter canonical frame semantics.
9. Application schemas cannot redefine reserved frame fields.
10. Hardware safety behavior is outside the communication library.

## Non-goals

- no cryptographic primitive implementation in Actus;
- no operating-system socket or filesystem implementation in `std::wire`;
- no transport-specific retry or MTU policy in the core;
- no application-specific command registry;
- no guarantee of real-time delivery or physical emergency-stop behavior;
- no assumption that a valid CRC proves identity or authorization.

## Acceptance evidence

The ADR is implemented only when the repository contains host, native, and
freestanding evidence for valid frames, malformed input, bounded arbitrary
input, zero-copy buffers, sequence handling, fragmentation, typed errors,
facade visibility, documentation, and source limits.
