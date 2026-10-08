# `std::region` runtime and target boundary

The public region API is transport-neutral. Hosted builds use the configured
runtime provider; freestanding builds require an explicit region provider
contract. The provider owns the backing implementation, but it must preserve
the public `Region[T]` descriptor, typed errors, generation checks, and
ownership cleanup.

The public read, write, publish, cancel, and close operations do not perform
filesystem I/O. A target adapter may use a device, mapped storage, or another
backend behind the private bridge, but that behavior is outside the language
surface and must not leak raw operating-system pointers into `Region[T]`.

The region descriptor is intentionally opaque. Native lowering may pass a
small descriptor or an indirect ABI representation according to the target;
source code must not depend on that representation.

For large logical extents, the compiler and runtime must keep layout metadata
symbolic and bounded. Logical capacity is not a promise that all elements are
resident in RAM at once.
