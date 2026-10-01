# ADR-0051: Universal Wire Codec and Ustari Communication Libraries

- Status: Proposed
- Date: 2026-10-01
- Scope: Actus target-neutral communication libraries

## Context

Actus needs a communication foundation that works on hosted systems,
freestanding targets, embedded devices, and future transports without coupling
the language or its standard library to an operating system, board, device
family, or application domain.

A byte buffer alone is not a protocol. Production communication requires a
bounded frame format, deterministic byte order, length validation, integrity
checking, incremental parsing, typed errors, versioning, sequence handling,
fragmentation rules, and explicit ownership of caller-provided storage. These
responsibilities must be reusable by more than one application protocol.

The design therefore separates two standard-library layers:

- `std::wire` provides the reusable binary transport foundation.
- `std::ustari` defines a universal, versioned communication protocol on top of
  that foundation.

Neither library may depend on a particular operating system, hardware device,
filesystem, UI, application, or external project. Transport adapters and
application-specific message families remain outside the protocol core.

## Decision

Actus will provide two target-neutral standard-library modules with explicit
facades and bounded contracts.

### `std::wire`

`std::wire` is the low-level codec layer. It owns:

- fixed-width little-endian integer encoding and decoding;
- canonical magic, version, flags, message, channel, sequence, and length
  fields;
- CRC16-CCITT integrity calculation and verification;
- bounded complete-frame serialization;
- incremental parsing from arbitrary input chunks;
- malformed-input classification and consumed-byte reporting;
- bounded fragmentation and reassembly primitives;
- caller-owned storage contracts for all temporary and output buffers.

The initial frame contract uses a fixed 12-byte header, a bounded payload, and
a 2-byte CRC trailer. The exact constants are named in the implementation and
are part of the protocol version. Multi-byte values have one explicit wire
byte order; host endianness must never affect serialized bytes.

The codec must validate the header and payload length before reading payload
bytes. It must reject unknown versions, reserved flags, impossible lengths,
invalid CRC values, truncated frames, and insufficient output capacity with
typed errors. A malformed frame must not panic, allocate without an explicit
caller contract, loop indefinitely, or access storage outside its bounds.

### `std::ustari`

`std::ustari` is the universal communication protocol layer. It owns:

- versioned message identifiers and bounded message schemas;
- logical channels and capability negotiation;
- authenticated session contracts;
- directional sequence handling and replay protection;
- typed acknowledgement and negative-acknowledgement messages;
- authorization decisions at the protocol boundary;
- duplicate, retry, timeout, and idempotency semantics;
- protocol-level error classification.

`std::ustari` consumes and produces `std::wire` frames. It does not implement
physical transport behavior. USB, UART, CAN, radio, TCP, and other adapters
feed byte chunks into the codec and own their own MTU, retry, timeout,
fragmentation scheduling, and connection lifecycle rules.

Authentication and encryption are exposed through a narrow typed facade. The
protocol library must not implement cryptographic primitives itself or select
an unaudited implementation implicitly. Encrypted payloads include their
authentication data in the bounded payload length, and the complete header is
covered by the authenticated-data contract.

### Actus ownership and allocation contract

The first implementation must support caller-owned storage:

- `ins Buffer` is used for mutable parser and reassembly storage;
- `abs Buffer` is used for read-only frame inspection and serialization input;
- returned buffers have explicit ownership and cleanup roles;
- parsing a complete caller-owned frame must support a zero-copy payload view;
- the core target-neutral path must not require an operating-system allocator.

Hosted convenience constructors may be added above the core contract, but
they must not change the bounded parser, error, or wire-format behavior.

### Module layout

The standard-library facades will expose a responsibility-oriented structure:

```text
library/std/src/wire/wire.act          # public facade
library/std/src/wire/frame.act         # frame model and constants
library/std/src/wire/codec.act         # bounded encode/decode operations
library/std/src/wire/parser.act        # incremental parser state
library/std/src/wire/checksum.act      # CRC16-CCITT
library/std/src/wire/fragment.act      # bounded fragmentation primitives
library/std/src/wire/error.act         # typed wire errors

library/std/src/ustari/ustari.act      # public facade
library/std/src/ustari/message.act     # versioned message metadata
library/std/src/ustari/session.act     # session and capability contracts
library/std/src/ustari/replay.act      # bounded replay window
library/std/src/ustari/authorization.act
library/std/src/ustari/error.act       # typed protocol errors
```

Only declarations exported by the canonical facades are public. Internal
helpers and future crypto or transport bridges remain private siblings.

## Invariants

1. `std::wire` has no dependency on an operating system, filesystem, host
   runtime, board, or transport implementation.
2. `std::ustari` has no dependency on a particular application or device
   domain.
3. Every frame has a bounded length, explicit version, and deterministic byte
   order.
4. Length and capacity checks occur before every payload or reassembly access.
5. Unknown versions, flags, channels, message identifiers, and schema fields
   fail closed with typed errors.
6. CRC provides integrity detection only; it is never treated as
   authentication or authorization.
7. Protected protocol operations require an established authenticated session
   and explicit authorization.
8. Replay state is bounded, directional, session-scoped, and reset only with
   a new authenticated session.
9. The core parser and serializer do not perform hidden allocation or
   unbounded work.
10. Transport adapters cannot alter the canonical frame semantics.
11. Application-specific schemas cannot silently redefine reserved protocol
    fields or security behavior.
12. Safety-critical local behavior is outside the communication protocol and
    cannot be delegated to a remote message.
13. The wire codec, protocol layer, and transport adapters have separate test
    and acceptance evidence.

## Consequences

### Positive consequences

- One bounded codec can support many transports and applications.
- Protocol behavior is testable on a host without pretending that host tests
  prove hardware transport behavior.
- Freestanding targets can use the same frame and schema rules without a host
  runtime or filesystem.
- Duplicated byte-order, length, CRC, and replay logic is avoided.
- Message schemas become versioned, reviewable, and independently testable.
- Actus ownership roles remain visible at the buffer boundary.

### Costs and risks

- The wire format and protocol schema require strict versioning discipline.
- Bounded storage requires explicit capacity, timeout, and reassembly policies.
- Authentication requires a separately reviewed, auditable crypto dependency.
- Each transport needs independent MTU, retry, timeout, and integration tests.
- The protocol core must resist pressure to absorb application-specific
  convenience features.

## Non-goals

- This ADR does not define a particular physical transport.
- This ADR does not define a device-specific command set.
- This ADR does not implement cryptographic primitives.
- This ADR does not provide a filesystem, allocator, scheduler, or UI.
- This ADR does not claim real-time delivery or emergency-response guarantees
  for remote communication.
- This ADR does not make CRC a security mechanism.

## Acceptance criteria

The ADR is implemented only when the following evidence exists:

- host round-trip tests for valid frames and schemas;
- malformed-input tests for truncation, invalid flags, lengths, versions, and
  CRC values;
- bounded arbitrary-input tests with no panic or unbounded loop;
- zero-copy and caller-storage tests for the core parser;
- replay, duplicate, stale-sequence, and session-reset tests;
- transport-independent fragmentation and reassembly tests;
- freestanding target-object evidence showing no hosted runtime dependency;
- deterministic formatter, LSP, and documentation behavior for public
  `wire` and `ustari` declarations.
