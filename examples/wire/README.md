# `std::wire` example

Build and run from this directory:

```sh
actus build --strict --emit exe -o wire-example
./wire-example
```

Expected output:

```text
WIRE_DEMO frame=encoded-decoded sequence=42 payload=ABC
WIRE_DEMO parser=split-chunks status=ready
```

The example exercises frame serialization, CRC validation through decoding,
sequence admission, and incremental parsing with two caller-owned chunks. It
does not open a socket or select a physical transport.
