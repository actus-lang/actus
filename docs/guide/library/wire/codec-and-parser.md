# Codec and incremental parser

## Header codec

`wire_header_encode` writes exactly 12 bytes into an existing output buffer.
It rejects insufficient capacity and invalid header metadata. It does not grow
the buffer.

`wire_header_decode` requires at least 12 input bytes, reads little-endian
sequence and length fields, and returns a `WireHeader` only after field
validation. Short input returns `Truncated`.

## Complete frame codec

`wire_frame_encode` checks that the payload buffer length equals
`header.payload_len`, validates output capacity, writes the header and payload,
then appends CRC. Its returned count includes the trailer.

`wire_frame_decode` checks the complete frame length and checksum before
copying into the caller's `ins payload_output`. A destination that is too
small returns `InsufficientCapacity`.

## Parser state

`WireParser` owns one bounded 1038-byte frame buffer. Its public lifecycle is:

1. `Searching`: ignore noise until `magic_0` appears.
2. partial magic/header: retain bounded bytes and continue across feeds.
3. `Collecting`: header is valid and expected frame length is known.
4. `Ready`: the complete frame passed decode and payload was copied.

Call `wire_parser_reset` after consuming a ready frame or after an application
chooses to abandon it.

## Split chunks and resynchronization

`wire_parser_feed` accepts any chunk size, including one byte. It reports the
number consumed from the latest chunk. Noise is skipped while searching. A
new magic byte can restart the partial magic match, allowing resynchronization
without an unbounded scan.

Unsupported version, unknown flags, oversized payload, truncated data, and bad
checksum return typed `WireError` values and reset the parser to a safe search
state where required by the implementation.

## Ownership

The parser is mutated through `ins`. Input chunks are borrowed through `abs`.
Payload output is an existing caller buffer loaned through `ins`; the parser
never returns hidden storage or retains the input chunk after the feed call.
