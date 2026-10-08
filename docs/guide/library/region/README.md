# `std::region`

`std::region` is a bounded logical storage capability. It represents a logical
sequence of fixed-layout elements while keeping only a checked resident window
available to the runtime. It is a distinct resource type; it is not an alias
for `Array[T, N]`, a raw pointer, or a filesystem mapping.

```actus
import std::region;
```

## What the module provides

- generic `Region[T]` ownership with compiler-checked element layout;
- explicit logical length and resident-window coordinates;
- checked read and write of one element at a time;
- explicit publish and cancel lifecycle operations;
- generation-aware close and cleanup;
- typed `RegionError` failures;
- hosted and target-provider boundaries without filesystem I/O in the public
  operations.

## Reading order

1. [Representation and opening](representation.md)
2. [Public API](api.md)
3. [Ownership and lifecycle](lifecycle.md)
4. [Bounds, generations, and errors](contracts.md)
5. [Runtime boundary](runtime.md)
6. [Examples](examples.md)

## Core distinction

`logical_length` describes the addressable logical sequence. `window_start` and
`window_count` describe the resident part available for the current operation.
A valid logical index can still return `OutOfWindow` when it is outside the
active resident window.

## Detailed pages

- [Usage guide](usage.md)
- [Region implementation](../../../../library/std/src/region/)
- [Region tests](../../../../tests/std_region_native.rs)

## Lifecycle

1. Call `region_open` with caller-owned backing storage and a fixed-layout
   element type.
2. Read or write only through indices accepted by the active resident window.
3. Call `region_publish` to commit a valid window mutation or `region_cancel`
   to discard it.
4. Treat a changed generation as invalidating older views and handles.
5. Call `region_close` when the region is no longer needed.

`Region[T]` is a bounded language-level capability. It is not an operating
system pointer and does not make an unsized or dynamically laid-out element
valid.
