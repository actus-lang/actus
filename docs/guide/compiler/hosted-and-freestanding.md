# Hosted and freestanding builds

A hosted build uses services supplied by the host runtime, such as console,
filesystem, or platform clocks.

A freestanding build has no implicit host service. It uses only the runtime
providers and public modules explicitly supplied by its target contract.

The compiler rejects an unavailable hosted module before native emission. Do not
assume that a source program that runs on a desktop has a complete embedded or
bare-metal target implementation.
