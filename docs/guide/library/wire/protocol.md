# `std::wire` protocol

## Version-one frame

The fixed frame layout is:

| Bytes | Field |
| ---: | --- |
| 0..1 | Magic `0xDA11`, stored as two bytes. |
| 2 | Version `1`. |
| 3 | Flags. |
| 4 | Message identifier. |
| 5 | Channel identifier. |
| 6..9 | Little-endian `u32` sequence number. |
| 10..11 | Little-endian `u16` payload length. |
| 12.. | Payload, at most 1024 bytes. |
| final 2 | Little-endian CRC16-CCITT. |

The maximum complete frame is 1038 bytes. CRC16 uses polynomial `0x1021`,
initial value `0xFFFF`, no reflection, and zero final XOR. CRC detects
accidental corruption; it is not authentication.

## Header and frame operations

`wire_header_encode` and `wire_header_decode` handle the fixed header.
`wire_frame_encode` copies a payload, computes CRC, and writes the complete
frame. `wire_frame_decode` validates header, length, and CRC before copying the
payload to caller-owned storage.

Every output buffer must already have enough capacity. Wire never grows a
buffer and never contacts a transport.
