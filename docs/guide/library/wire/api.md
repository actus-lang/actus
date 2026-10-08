# `std::wire` public API

The public facade exports the following contracts. Private helper verbs and
raw runtime bridges are not importable API.

## Frame constants and `WireHeader`

```actus
const WIRE_HEADER_BYTES: u32 = 12u32;
const WIRE_CRC_BYTES: u32 = 2u32;
const WIRE_MAX_PAYLOAD_BYTES: u32 = 1024u32;
const WIRE_MAX_FRAME_BYTES: u32 = 1038u32;

pack WireHeader {
    storage: Array[u8, 12],
    magic_0: u8,
    magic_1: u8,
    version: u8,
    flags: u8,
    message_id: u8,
    channel_id: u8,
    sequence_num: u32,
    payload_len: u16,
}
```

The public constants also include magic bytes `0xDA` and `0x11`, version `1`,
CRC16 polynomial `0x1021`, initial CRC `0xFFFF`, and known flags for
acknowledgement, urgency, and negotiated compression.

## Errors

```actus
enum WireError {
    InvalidMagic,
    UnsupportedVersion,
    UnknownFlags,
    PayloadTooLarge,
    Truncated,
    InsufficientCapacity,
    PayloadLengthMismatch,
    ChecksumMismatch,
    InvalidRange,
    PolicyRejected,
    SequenceDuplicate,
    SequenceStale,
    SequenceJumpTooLarge,
    SequenceContextMismatch,
    FragmentUnsupportedVersion,
    FragmentCountInvalid,
    FragmentIndexInvalid,
    FragmentRangeInvalid,
    FragmentTooLarge,
    FragmentDuplicate,
    FragmentOverlap,
    FragmentStale,
    ReassemblyCapacity,
    ReassemblyState,
    ReassemblyIncomplete,
}
```

## Checksum and frame codec

```actus
verb wire_crc16_ccitt(abs input: Buffer, erg start: u32, erg end: u32)
    -> Result[u16, WireError];
verb wire_header_encode(abs header: WireHeader, ins output: Buffer)
    -> Result[u32, WireError];
verb wire_header_decode(abs input: Buffer)
    -> Result[WireHeader, WireError];
verb wire_frame_encode(abs header: WireHeader, abs payload: Buffer,
    ins output: Buffer) -> Result[u32, WireError];
verb wire_frame_decode(abs input: Buffer, ins payload_output: Buffer)
    -> Result[WireHeader, WireError];
```

The frame encoder returns the complete byte count including CRC. The decoder
validates header fields, payload bounds, and checksum before copying payload.

## Incremental parser

```actus
enum WireParserStatus { Searching, Collecting, Ready }
struct WireParser { storage: Buffer, filled: u32, expected_length: u32,
    state: u8, last_consumed: u32, header: WireHeader }
verb wire_parser_empty() -> WireParser;
verb wire_parser_reset(ins parser: WireParser) -> Void;
verb wire_parser_status(abs parser: WireParser) -> WireParserStatus;
verb wire_parser_consumed(abs parser: WireParser) -> u32;
verb wire_parser_header(abs parser: WireParser) -> WireHeader;
verb wire_parser_feed(ins parser: WireParser, abs chunk: Buffer,
    ins payload_output: Buffer) -> Result[Int, WireError>;
```

The parser accepts arbitrary chunks and reports bytes consumed from the latest
feed. A ready parser retains one validated frame until reset.

## Sequence and fragments

```actus
struct WireSequenceWindow {
    initialized: Bool,
    context_id: u32,
    highest: u32,
    received: u64,
}
verb wire_sequence_empty() -> WireSequenceWindow;
verb wire_sequence_accept(ins window: WireSequenceWindow,
    erg context_id: u32, erg sequence_num: u32) -> Result[Int, WireError>;
verb wire_sequence_reset(ins window: WireSequenceWindow) -> Void;
verb wire_sequence_replace_context(ins window: WireSequenceWindow,
    erg context_id: u32) -> Void;
verb wire_sequence_highest(abs window: WireSequenceWindow) -> u32;
```

Fragment types and `WireReassembly` are documented in [fragments](fragments.md).

## Optional endpoint

`wire_endpoint_parse(abs input: Buffer)` returns a fixed scalar offset view.
It is a convenience parser, not a transport connector.
