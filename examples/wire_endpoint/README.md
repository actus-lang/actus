# `std::wire` endpoint example

This example demonstrates the optional hosted `wire://` endpoint layer. It
stores `wire://robot:8080/control` in caller-owned bytes, parses the scheme,
authority, port, and path, and prints a confirmation only after the returned
metadata matches those values.

The parser does not open a socket, select a transport, allocate a String, or
change the `std::wire` frame path. Embedded users can omit this endpoint layer.

Run it from this directory with:

```sh
actus build --strict --emit exe -o /tmp/wire-endpoint-example
/tmp/wire-endpoint-example
```

Expected output:

```text
WIRE_ENDPOINT_DEMO parsed authority=robot port=8080 path=/control
WIRE_ENDPOINT_DEMO transport=not-selected core=frame-independent
```
