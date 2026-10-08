# `std::region` representation and opening

## `Region[T]`

`Region[T]` is an opaque compiler and runtime-managed descriptor. Application
code does not construct its fields and does not inspect an operating-system
pointer. The type parameter `T` determines the element stride and alignment
through the compiler's fixed-layout `size_of[T]()` and `align_of[T]()` contracts.

Supported element categories are fully sized primitives, fixed arrays, packs,
and structs whose fields have a fixed native layout. Unsized, incomplete, or
dynamically laid-out elements are rejected before native emission.

## Opening a region

```actus
open verb region_open[T](
    dat backing: Buffer,
    erg logical_length: u64,
    erg window_start: u64,
    erg window_count: u64
) -> Result[Region[T], RegionError];
```

`backing` is transferred to the opened capability after validation. The
runtime checks the element stride, element alignment, logical extent, resident
window, and backing capacity. A failed open returns `RegionError` and does not
return a usable region owner.

The logical extent may be much larger than the resident storage. The region
does not automatically load or save external data; target adapters define how
resident storage is supplied.

## Region versus array

Use `Array[T, N]` when all `N` elements are inline in one Actus value and
compile-time extent is part of the type. Use `Region[T]` when logical extent
and resident window must be separate runtime metadata. There is no implicit
conversion or implicit indexed syntax between the two types.
