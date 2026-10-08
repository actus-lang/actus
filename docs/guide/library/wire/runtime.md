# Runtime and transport boundary

## No transport dependency

`std::wire` is a byte protocol library, not a socket or device library. It has
no filesystem, operating-system, network, scheduler, cryptographic, or UI
dependency. A transport adapter owns reads, writes, framing queues, timeouts,
and connection lifecycle.

The wire codec can therefore be used by desktop applications, embedded
firmware, shared-memory adapters, test fixtures, or a later operating system
without changing the frame contract.

## Bounded storage

The parser owns one maximum-frame buffer. Reassembly uses caller-provided
storage and fixed arrays for fragment ranges. Sequence protection uses one
64-bit mask. Endpoint parsing scans only bounded input lengths. There is no
hidden heap allocation or unbounded loop contract.

## Integrity and security

CRC16 is accidental-corruption detection only. It cannot provide encryption,
authentication, authorization, replay protection by itself, or sender
identity. The sequence window rejects duplicates and stale values in one
context, but a higher-level authenticated session is required for adversarial
networks.

## Addressing

`wire://` is optional syntax for host-side configuration and diagnostics. It
is not embedded wire framing and is not required on microcontrollers. Numeric
header identifiers and adapter-specific addressing remain valid without it.

## Verification boundary

Native tests prove frame encoding/decoding, deterministic CRC, split parser
chunks, endpoint bounds, sequence-window behavior, and bounded fragment
reassembly. They do not prove a particular UART, socket, radio, encryption
layer, or physical link.
