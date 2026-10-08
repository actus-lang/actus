# `std::region` example

This package demonstrates the public Region lifecycle with an inline array
element type. It opens one resident window, performs a bounded bulk write and
read, publishes a mutation, cancels a later mutation, remaps the clean window,
and closes the capability.

Build and run from this directory:

```sh
actus build --strict --emit exe -o region-example
./region-example
```

The program returns exit status `0` when every checked operation succeeds. It
does not perform provider I/O; provider recovery and corruption fallback are
specified and tested at the runtime-provider boundary.

