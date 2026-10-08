# Using `std::wire`

## Encode a frame

Build a validated `WireHeader`, prepare a caller-owned output buffer large
enough for header, payload, and CRC, then call `wire_frame_encode`:

```actus
open verb send_frame(abs header: WireHeader, abs payload: Buffer,
    ins output: Buffer) -> Result[u32, WireError] {
    return wire_frame_encode(
        header: abs header,
        payload: abs payload,
        output: ins output,
    );
}
```

The transport adapter sends only the returned byte range.

## Decode a complete frame

Provide an input buffer containing one complete frame and an output buffer with
capacity for the declared payload. `wire_frame_decode` validates before
copying. Inspect the returned header only on `Ok`.

## Feed a stream incrementally

Create one parser per input stream or frame consumer. Feed chunks as they
arrive, inspect `wire_parser_status`, and reset after consuming `Ready`:

```actus
open verb feed(ins parser: WireParser, abs chunk: Buffer, ins payload: Buffer)
    -> Result[Int, WireError] {
    return wire_parser_feed(
        parser: ins parser,
        chunk: abs chunk,
        payload_output: ins payload,
    );
}
```

The caller owns transport reads and decides how to handle `last_consumed`.

## Apply sequence policy

After a decoded header passes any higher-level session checks, call
`wire_sequence_accept` with the context and sequence. Reject typed duplicate,
stale, jump, or context errors before dispatching the message.

## Reassemble fragments

Open reassembly over caller storage, begin a generation, accept fragments, and
copy only after the result reports complete. Cancel a partial generation when
the session expires or a new generation replaces it.

## Endpoint convenience

On a desktop, call `wire_endpoint_parse` on a bounded byte buffer if a
`wire://` address is useful in configuration. Keep the input alive while using
the returned offsets. Embedded adapters may omit this operation entirely.
