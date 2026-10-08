# `std::wire` examples

## Encode with caller-owned storage

```actus
import std::wire;

open verb encode(abs header: WireHeader, abs payload: Buffer, ins output: Buffer)
    -> Result[u32, WireError] {
    return wire_frame_encode(
        header: abs header,
        payload: abs payload,
        output: ins output,
    );
}
```

The caller sends only the returned byte count through its selected adapter.

## Incremental parser

```actus
open verb consume(ins parser: WireParser, abs chunk: Buffer, ins payload: Buffer)
    -> Result[Int, WireError] {
    erg result = wire_parser_feed(
        parser: ins parser,
        chunk: abs chunk,
        payload_output: ins payload,
    );
    return result;
}
```

Chunks may split the magic prefix, header, payload, or CRC. The parser retains
bounded state between calls.

## Replay-window check

```actus
open verb accept_frame(ins window: WireSequenceWindow, erg context: u32,
    erg sequence: u32) -> Result[Int, WireError] {
    return wire_sequence_accept(
        window: ins window,
        context_id: erg context,
        sequence_num: erg sequence,
    );
}
```

Only dispatch a decoded message after this policy accepts its sequence.

## Optional endpoint parsing

```actus
open verb parse_address(abs bytes: Buffer)
    -> Result[WireEndpoint, WireEndpointError] {
    return wire_endpoint_parse(input: abs bytes);
}
```

This parses bounded configuration bytes. It does not connect, send, receive,
or require all wire users to use an address string.

## Fragment reassembly

```actus
open verb accept_message(ins reassembly: WireReassembly, abs fragment: Buffer)
    -> Result[Int, WireError] {
    return wire_reassembly_accept(
        reassembly: ins reassembly,
        fragment: abs fragment,
    );
}
```

A return value of `Ok(1)` means all declared fragments have arrived; `Ok(0)`
means the generation is still collecting.
