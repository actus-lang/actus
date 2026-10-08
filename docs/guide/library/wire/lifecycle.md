# Ownership and lifecycle

## Buffer roles

- `abs` frame, payload, input, and endpoint buffers are borrowed read-only.
- `ins` output, parser, sequence window, and reassembly state are mutated in
  caller-owned storage for the duration of a call.
- `dat` reassembly storage transfers ownership into `WireReassembly`.

The codec never allocates a replacement buffer when output capacity is too
small. The caller chooses and provisions storage before encoding, feeding, or
reassembly.

## Parser lifecycle

Create with `wire_parser_empty`, feed chunks until `Ready`, read the header
and copied payload, then reset before accepting another frame. A parser may be
reset after a malformed frame or abandoned partial input.

## Sequence lifecycle

Create an empty window, establish a context on the first accepted sequence,
and use explicit reset or context replacement when the stream changes. A
rejected duplicate, stale, or mismatched frame does not mutate the window.

## Reassembly lifecycle

`wire_reassembly_open` consumes caller storage. `wire_reassembly_begin`
starts one generation after checking capacity. Accepted fragments are copied
into that storage. `wire_reassembly_copy` borrows the completed state; it does
not consume it. `wire_reassembly_cancel` clears metadata and keeps storage
owned by the reassembly value for reuse.

## Endpoint view lifetime

`WireEndpoint` contains offsets into the input buffer, not owned endpoint text.
The input must remain available while those offsets are interpreted. Do not
serialize the offset view without also defining the lifetime and identity of
the source bytes.
