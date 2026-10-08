# `std::wire` examples

## Encode a frame

```actus
import std::wire;

verb encode(abs header: WireHeader, abs payload: Buffer, ins output: Buffer) -> Result[u32, WireError] {
    return wire_frame_encode(header: abs header, payload: abs payload, output: ins output);
}
```

The caller supplies output capacity for the header, payload, and two CRC
bytes. The function does not allocate or send the frame.

## Feed split input

```actus
verb feed(ins parser: WireParser, abs chunk: Buffer, ins payload: Buffer) -> Result[u32, WireError] {
    return wire_parser_feed(parser: ins parser, input: abs chunk, payload_output: ins payload);
}
```

Call `wire_parser_feed` again with the next chunk while the parser remains in
`Collecting`. Reset it after `Ready` or after a handled error.
