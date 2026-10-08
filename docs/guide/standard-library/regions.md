# `std::region`

```actus
import std::region;
```
`std::region` provides bounded logical-region operations. A logical capacity
can be larger than the resident window; access remains checked against the
active descriptor, window, generation, and element layout.

The public facade exports region errors and checked lifecycle operations. A
region element must have a fixed native layout. Views and handles are bounded
capabilities, not raw operating-system pointers.

Publication, cancellation, close, generation changes, and cleanup are explicit
lifecycle operations. Use the typed result of each operation and do not retain
a view after its generation or region has become invalid.

## Public surface

The facade includes:

- `Region[T]` and `RegionError`;
- `region_open` for a fixed-layout element, logical length, and resident window;
- `region_read` and `region_write` for checked indexed access;
- `region_publish` and `region_cancel` for window lifecycle;
- `region_close` for deterministic cleanup.

Opening a region consumes the backing storage according to its contract.
Reading uses an `abs` region and caller-provided destination storage. Writing,
publishing, cancelling, and closing use the required exclusive role. Bounds,
layout, generation, capacity, and lifecycle failures remain typed.
