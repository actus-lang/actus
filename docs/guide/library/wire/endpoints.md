# Optional endpoint parsing

The endpoint parser recognizes a bounded byte form:

```text
wire://authority[:port][/path]
```

It is an optional hosted convenience for desktop applications, diagnostics,
or configuration. It is not required for frame exchange and does not open a
transport.

## Bounds

- complete input: at most 256 bytes;
- authority: at most 128 bytes;
- path: at most 128 bytes;
- explicit port: decimal and within `1..=65535`.

The parser accepts the exact seven-byte `wire://` scheme. Authority must be
present and cannot contain whitespace, `/`, `:`, `?`, or `#`. Path bytes must
be printable non-zero bytes under the module's bounded validation rules.

## Returned view

`WireEndpoint` stores scalar offsets and lengths into the unchanged input:

- source length;
- authority offset and length;
- port and `has_port` flag;
- path offset and length;
- validity marker.

It does not copy the input and does not own the bytes. The input buffer must
remain valid while consumers use its offsets.

## Errors

`TooLong`, `InvalidScheme`, `MissingAuthority`, `InvalidAuthority`,
`InvalidPort`, `PortOutOfRange`, and `InvalidPath` identify separate parsing
failures. No endpoint error opens a socket, resolves a name, or chooses a
transport.

## Embedded usage

Embedded code can omit endpoint parsing completely. It can send frames through
a numeric channel, message identifier, sequence number, and caller-provided
buffers. This keeps strings out of the wire core and leaves addressing to the
adapter or host-side application.
