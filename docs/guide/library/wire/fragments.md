# Fragmentation and reassembly

Fragmentation carries a message larger than one frame payload while keeping
all storage bounded and caller-owned.

## Fragment header

Each fragment has a fixed 17-byte metadata prefix:

| Offset | Field | Size |
| ---: | --- | ---: |
| 0 | version | 1 byte |
| 1..2 | fragment index | little-endian `u16` |
| 3..4 | fragment count | little-endian `u16` |
| 5..8 | total message length | little-endian `u32` |
| 9..12 | fragment offset | little-endian `u32` |
| 13..16 | generation | little-endian `u32` |

The protocol accepts at most 64 fragments, each fragment payload is bounded by
1024 bytes, and a reassembled message is bounded by 4096 bytes.

## Encoding

`wire_fragment_encode` validates version, count, index, total length, offset,
payload size, and output capacity before writing metadata and payload. The
input is borrowed and the output is an existing `ins Buffer`.

`wire_fragment_decode` requires the 17-byte prefix and validates the decoded
metadata against the remaining payload length. It returns a typed error before
publishing invalid metadata.

## Reassembly lifecycle

1. `wire_reassembly_open(dat storage)` transfers caller storage into a bounded
   reassembly state.
2. `wire_reassembly_begin(ins reassembly, total_length, fragment_count,
   generation)` checks declared limits and storage capacity.
3. `wire_reassembly_accept(ins reassembly, abs fragment)` validates identity,
   generation, index, range, duplicate, and overlap, then copies the payload.
4. When all fragments arrive, `wire_reassembly_copy(abs reassembly, ins output)`
   copies the complete message.
5. `wire_reassembly_cancel(ins reassembly)` clears lifecycle counters while
   retaining the caller's storage for reuse.

## Safety rules

A fragment from another generation returns `FragmentStale`. A repeated index
returns `FragmentDuplicate`; an intersecting byte range returns
`FragmentOverlap`. A message cannot be copied before all declared fragments
arrive and output capacity is checked before copying.

Reassembly does not allocate, perform I/O, or accept a fragment after the
state is idle or complete. The caller controls when a generation begins and
ends.
