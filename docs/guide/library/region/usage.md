# `std::region` usage

## Open a region

`region_open[T]` receives caller-owned backing storage, a logical length, a
resident window start, and a resident window count. `T` must have a fixed
native layout supported by the compiler.

```actus
import std::region;

verb open_bytes(dat storage: Buffer) -> Result[Region[u8], RegionError] {
    return region_open[u8](
        backing: dat storage,
        logical_length: 1024u64,
        window_start: 0u64,
        window_count: 64u64
    );
}
```

The logical length can exceed the resident window. Access is still checked
against the active descriptor and resident bounds.

## Read and write

`region_read` copies one checked element into caller-owned destination storage.
`region_write` validates the source representation and writes one element into
the active region. Neither operation expands the region or changes its logical
length.
