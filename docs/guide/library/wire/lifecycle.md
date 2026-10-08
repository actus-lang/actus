# `std::wire` lifecycle

## Incremental parsing

Create a `WireParser`, feed arbitrary chunks, inspect `consumed`, and check
`status`. A split header or payload is normal. When the parser reaches `Ready`,
read the validated header and copied payload, then reset before feeding the
next frame.

## Sequence admission

`WireSequenceWindow` is scoped by an explicit context. The first sequence
establishes the window; forward sequences advance it; duplicates and stale
values are rejected. Replacing the context clears the prior history. The
window is bounded and uses a 64-frame bitmask.

## Fragment reassembly

Open reassembly with caller-owned storage, begin a generation with total
length and fragment count, then accept each encoded fragment. Duplicate,
overlapping, stale, out-of-range, and capacity-violating fragments are
rejected. Copy the complete payload only after all fragments arrive, or call
cancel to discard the lifecycle.

## Endpoint metadata

`wire_endpoint_parse` is optional hosted metadata for bounded
`wire://authority[:port][/path]` values. It returns offsets into the unchanged
input buffer and does not open a socket. Embedded users can omit endpoint
parsing and exchange frames through numeric channel identifiers.
